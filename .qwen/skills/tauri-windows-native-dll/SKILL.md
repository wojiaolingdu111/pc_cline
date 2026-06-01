---
name: tauri-windows-native-dll
description: 在 Tauri v2 Windows 打包中捆绑原生 DLL（如 LibTorch/tch-rs 的 torch_cpu.dll），含 DELAYLOAD + 三层兜底
source: auto-skill
extracted_at: '2026-06-01T07:01:19.476Z'
---

# Tauri v2 Windows 原生 DLL 捆绑

## 问题

当 Rust 依赖通过 FFI 链接原生 DLL（如 `tch-rs` → `torch-sys` → LibTorch 的 `torch_cpu.dll`），即使启用了 `download-libtorch` feature，运行时仍然会报 "找不到 torch_cpu.dll"。

**根因一：DLL 在 process startup 被加载**
`torch-sys` 通过 `cargo:rustc-link-lib=torch_cpu` 生成**导入表条目**，Windows 加载器在**进程启动时**就解析它 —— 比 `main()` 和 Tauri `setup()` 都早。任何在 `setup()` 里做 `AddDllDirectory` / 修改 `PATH` 的尝试都**为时已晚**。

**根因二：`SetDefaultDllDirectories` 移除 PATH 搜索**
调用 `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_APPLICATION_DIR | ...)` 后，Windows 只搜索 exe 目录 + 系统目录 + `AddDllDirectory` 注册的目录，**PATH 被移出搜索列表**。后续修改 PATH 对当前进程的 `LoadLibrary` 无效。

## 解决方案：DELAYLOAD + 三层兜底

### 核心思路

用 MSVC 的 `/DELAYLOAD` 标志把 DLL 从进程启动加载改为**延迟加载**（首次调用 torch 函数时触发）。这样 `setup()` 就有时间设置搜索路径。

### 0. build.rs — DELAYLOAD（核心修复）

在 `build.rs` 的 `main()` 中，在 `tauri_build::build()` 之前添加：

```rust
// 仅 Windows MSVC：延迟加载 LibTorch DLL，避免 process startup crash
#[cfg(all(target_os = "windows", target_env = "msvc"))]
{
    println!("cargo:rustc-link-arg=-DELAYLOAD:torch_cpu.dll");
    println!("cargo:rustc-link-arg=-DELAYLOAD:c10.dll");
    println!("cargo:rustc-link-arg=-DELAYLOAD:torch.dll");
    println!("cargo:warning=delay-load enabled for LibTorch DLLs on Windows MSVC");
}
```

**为什么要 delay 这三个 DLL？**
- `torch-sys` 的 `build.rs` 输出 `cargo:rustc-link-lib=torch_cpu` / `c10` / `torch`，这三者直接出现在导入表
- `torch_global_deps` 是被 `torch_cpu.dll` 自己加载的，不需要 delay

### 1. build.rs — 构建时复制 DLL（增强版 find_libtorch_lib_dir）

`torch-sys` 的 `build.rs` 会把 LibTorch 下载到 `target/<profile>/build/torch-sys-{hash}/out/libtorch/libtorch/lib/`。我们需要把 DLL 从那里复制到两个目标：
- **`target/<profile>/`** — 开发模式（`tauri dev`），exe 同目录
- **`src-tauri/libtorch-dlls/`** — 供 Tauri 打包使用

增强版 `find_libtorch_lib_dir` 应支持 5 种回退（按优先级）：

```rust
fn find_libtorch_lib_dir(profile_dir: &Path) -> Option<PathBuf> {
    // 1. cargo metadata: DEP_TORCH_SYS_LIBTORCH_LIB (Linux/macOS)
    //    DEP_TCH_LIBTORCH_LIB, DEP_TORCH_SYS_LIB_DIR 等
    //
    // 2. LIBTORCH 环境变量 → <LIBTORCH>/lib/
    //
    // 3. LIBTORCH_LIB 环境变量（直接指向 lib 目录）
    //
    // 4. 搜索 target/<profile>/build/ 下 torch-sys-*/ 和 tch-*/ 目录
    //    的子目录：out/libtorch/libtorch/lib/ 或 out/libtorch/lib/ 或 out/
    //
    // 5. 系统级路径回退（Linux: /usr/lib/libtorch.so 等）
}
```

此函数**必须**同时检查 `.dll`（Windows）、`.so`（Linux）、`.dylib`（macOS）。

```rust
fn platform_libtorch_marker() -> &'static str {
    if cfg!(target_os = "windows") { "torch_cpu.dll" }
    else if cfg!(target_os = "macos") { "libtorch.dylib" }
    else { "libtorch_cpu.so" }
}
```

### 2. tauri.conf.json — bundle.resources

```json
"bundle": {
    "resources": {
        "libtorch-dlls/*": ".."
    }
}
```

`".."` 含义：相对于资源目录上一级。在 Windows MSI 中，这会把 DLL 放到 `{InstallDir}/`（exe 同级或 resources/ 同级，取决于具体 SDK 版本）。配合第 3 步的 PATH 修正，无论实际落盘位置都能找到。

### 3. lib.rs — 启动时修改 PATH + 紧急拷贝（兜底）

⚠️ **关键警告**：**不要**在兜底代码中调用 `SetDefaultDllDirectories`！它会把 PATH 从搜索顺序中移除。

```rust
#[cfg(target_os = "windows")]
fn ensure_libtorch_dlls_searchable(app: &tauri::App) {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // exe 目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    // 资源目录
    if let Ok(d) = app.path().resource_dir() {
        dirs.push(d);
    }

    if dirs.is_empty() { return; }

    // --- 方法 A：修改 PATH ---
    // 把 exe_dir 和 resource_dir 前置到 PATH
    // 标准 LoadLibraryEx 搜索包含 PATH，DELAYLOAD helper 用的是标准搜索
    let paths_to_add: Vec<String> = dirs.iter()
        .filter_map(|d| d.to_str()).map(|s| s.to_owned()).collect();

    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut parts: Vec<&str> = current_path.split(';').collect();
    for p in &paths_to_add {
        if !parts.contains(&p.as_str()) {
            parts.insert(0, p);
        }
    }
    std::env::set_var("PATH", &parts.join(";"));

    // --- 方法 B：紧急拷贝 DLL 到 exe 目录 ---
    // 这是为了让「下一次启动」完全不依赖 PATH
    if dirs.len() >= 2 {
        let exe_dir = &dirs[0];
        let res_dir = &dirs[1];
        if exe_dir != res_dir && res_dir.exists() {
            for entry in std::fs::read_dir(res_dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "dll") {
                    let dest = exe_dir.join(path.file_name().unwrap());
                    if !dest.exists() {
                        std::fs::copy(&path, &dest).ok();
                    }
                }
            }
        }
    }
}
```

在 `setup` 闭包最开始调用（`ensure_libtorch_dlls_searchable(app)`），确保在第一个 Tauri command 处理之前 PATH 已修正。

## 完整数据流

```
tauri dev / tauri build
  ↓
build.rs: 复制 DLL 到 target/debug/（exe 同目录）
build.rs: 复制 DLL 到 src-tauri/libtorch-dlls/（打包用）
build.rs: 添加 DELAYLOAD 标志（Windows MSVC）
  ↓
进程启动 → Windows 加载器（跳过 torch_cpu.dll → DELAYLOAD 占位）
  ↓
Tauri setup() → ensure_libtorch_dlls_searchable()
  ├─ 把 exe_dir + resource_dir 前置到 PATH
  └─ 把 DLL 从 resource_dir 拷贝到 exe_dir（首次启动）
  ↓
用户触发操作 → 调用 torch 函数
  ↓
DELAYLOAD helper → LoadLibraryEx("torch_cpu.dll", NULL, 0)
  ↓
标准搜索：exe_dir → CWD → System32 → Windows → **PATH ✓**
  ↓
torch_cpu.dll 加载成功
```

## 调试建议

如果仍然报错，检查以下点：
1. **`build.rs` 的 `cargo:warning` 输出**：运行 `cargo build -v 2>&1 | grep find_libtorch` 确认找到路径
2. **`libtorch-dlls/` 目录内容**：在 Windows 上构建后检查该目录是否有 `.dll` 文件
3. **DLL 依赖链**：`torch_cpu.dll` 依赖 `c10.dll`、`torch.dll`、`torch_global_deps.dll`，确保全部在
4. **DELAYLOAD 是否生效**：用 [Dependency Walker](https://www.dependencywalker.com/) 或 [Dependencies](https://github.com/lucasg/Dependencies) 检查 exe 的导入表

## 适用场景

- `tch-rs` / `torch-sys` 的 LibTorch DLL
- 任何编译时通过 `cargo:rustc-link-lib` 依赖、需要在运行时加载的原生 DLL
- Windows MSVC 目标（`x86_64-pc-windows-msvc`）
- 适用于 `tauri dev` 和 `tauri build` 两种模式
