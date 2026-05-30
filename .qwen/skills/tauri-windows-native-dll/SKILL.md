---
name: tauri-windows-native-dll
description: 在 Tauri v2 Windows 打包中捆绑原生 DLL（如 LibTorch/tch-rs 的 torch_cpu.dll）
source: auto-skill
extracted_at: '2026-05-30T14:39:53.935Z'
---

# Tauri v2 Windows 原生 DLL 捆绑

## 问题

当 Rust 依赖通过 FFI 动态加载原生 DLL（如 `tch-rs` → `torch-sys` → LibTorch 的 `torch_cpu.dll`），即使启用了 `download-libtorch` feature（编译时自动下载），Cargo 也**不会**自动把 DLL 打包进 Tauri 安装包。运行时找不到 DLL，应用崩溃。

## 解决方案：三层兜底

### 1. build.rs — 构建时复制 DLL

在 `src-tauri/build.rs` 中，利用 `torch-sys` 构建产物路径自动定位 DLL 并复制：

- 搜索 `target/<profile>/build/torch-sys-*/out/libtorch/libtorch/lib/` 
- 支持 `LIBTORCH` 环境变量回退
- 复制到两个目标：
  - `target/<profile>/` — `tauri dev` 开发模式
  - `src-tauri/<dll-dir>/` — 供打包使用（加到 `.gitignore`）

关键代码模式：

```rust
fn find_libtorch_lib_dir(profile_dir: &Path) -> Option<PathBuf> {
    // 1. LIBTORCH 环境变量
    // 2. target/<profile>/build/torch-sys-*/out/libtorch/libtorch/lib/
}
```

### 2. tauri.conf.json — bundle.resources 映射到 exe 同级

```json
"bundle": {
    "resources": {
        "<dll-dir>/*": ".."
    }
}
```

`".."` 表示相对于资源目录（`resources/`）的上级，即安装根目录，DLL 会被放到 exe 同级。

### 3. lib.rs — 启动时 AddDllDirectory（兜底）

万一 resources 没放到 exe 同级，程序启动时通过 Windows API 注册 DLL 搜索路径：

```rust
#[cfg(target_os = "windows")]
fn add_resource_dir_to_dll_search(app: &tauri::App) {
    // SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_USER_DIRS)
    // AddDllDirectory(资源目录)
    // AddDllDirectory(exe 所在目录)
}
```

在 `setup` 闭包最开始调用，保证任何 DLL 加载尝试之前路径已注册。

## 适用场景

- `tch-rs` / `torch-sys` 的 LibTorch DLL
- 任何编译时依赖但需要运行时加载的原生 DLL
- 适用于 `tauri dev` 和 `tauri build` 两种模式
