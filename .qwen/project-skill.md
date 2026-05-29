# AI ToReder — PC 端本地语音合成工作台

## 项目概述

AI ToReder 是一款纯本地的桌面端文字转语音（TTS）工作台。输入文本、选择音色，即可在本地生成语音音频，支持声音克隆。所有计算在本地完成，无需联网、不依赖 API。

## 技术栈

| 层 | 技术 |
|---|---|
| 前端框架 | Vue 3 + TypeScript + Vite 8 |
| 状态管理 | Pinia |
| 桌面壳 | Tauri 2 |
| TTS 引擎 | qwen3-tts-rs 0.2（Rust 原生，无 Python） |
| 深度学习后端 | tch-rs（LibTorch），GPU (CUDA) / CPU 自动检测 |
| 授权管理 | Rust 本地 trial + 远程授权服务器（Axum + SQLite） |
| 构建分发 | GitHub Actions → 七牛云 CDN |

## 目录结构

```
pc_clinet/
├─ src/                          # Vue 3 前端
│  ├─ api/tauri.ts               # Tauri IPC 桥接层
│  ├─ components/
│  │  ├─ TextInputPanel.vue      # 文本输入 + 语速/语言 + 生成按钮
│  │  ├─ VoiceSelector.vue       # 内置/自定义音色选择
│  │  ├─ AudioPlayer.vue         # 音频播放 + 下载
│  │  ├─ CloneVoicePanel.vue     # 声音克隆（上传参考音频 + 创建 profile）
│  │  └─ TaskList.vue            # 任务历史列表
│  ├─ stores/
│  │  ├─ tts.ts                  # TTS 状态
│  │  ├─ voices.ts               # 音色列表
│  │  └─ settings.ts             # 服务状态
│  ├─ App.vue                    # 根布局
│  ├─ main.ts                    # 入口
│  └─ style.css                  # 全局样式
│
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs                 # 入口
│  │  ├─ lib.rs                  # Tauri Builder + 命令注册
│  │  ├─ commands.rs             # 8 个 Tauri 命令（核心业务）
│  │  ├─ state.rs                # AppState + TtsEngine
│  │  ├─ file_manager.rs         # 数据目录管理
│  │  └─ license.rs              # 授权管理
│  ├─ Cargo.toml
│  └─ tauri.conf.json
│
├─ scripts/
│  ├─ ci/build_desktop.sh/.ps1   # CI 构建脚本
│  ├─ manage-keys.mjs            # 授权码管理 CLI
│  └─ release.sh                 # 一键发版
│
├─ .github/workflows/build.yml   # CI/CD
├─ config/secrets.env.example
├─ Makefile
├─ package.json
└─ vite.config.ts
```

## 核心业务逻辑

### 语音合成流程

```
用户输入文本 → 选择音色 → 点击"开始生成"
  ↓
TextInputPanel.vue → ttsStore.submitGeneration(voiceId)
  ↓
tauri.ts (invoke IPC) → commands.rs::generate_speech()
  ↓
① 尝试懒加载模型
② 判断内置/自定义音色
   - 内置 → model.generate_custom_voice(text, Speaker, lang, ...)
   - 自定义 → 读取 profiles 元信息 → model.generate_voice_clone(...)
③ 写入 WAV 文件: outputs/{taskId}.wav
④ 返回 { taskId, status, audioPath, durationMs }
  ↓
AudioPlayer 播放 + 下载
```

### 声音克隆流程

```
用户点击"浏览…" → pick_audio_file（原生文件对话框）
  ↓
填写名称、选择语言 → 点击"创建音色 Profile"
  ↓
commands.rs::clone_voice()
  ↓
① 生成 ID: voice-user-{timestamp_ms}
② 复制参考音频到 voices/profiles/{id}.{ext}
③ 保存元信息 JSON
④ 返回 voiceProfileId
  ↓
刷新音色列表 → 自动选中新音色
```

### 授权系统

- **本地端**: 7 天试用，远程激活，状态持久化到 license.json
- **远程服务器**: Axum + SQLite，独立部署，管理授权码

### 浏览器预览模式

检测 `__TAURI_INTERNALS__`：非 Tauri 环境返回 mock 数据，前端可独立开发。

### Rust 命令清单（8 个）

| 命令 | 功能 |
|---|---|
| generate_speech | 语音合成（内置/克隆音色） |
| list_voices | 列出内置 + 自定义音色 |
| clone_voice | 声音克隆，创建 profile |
| delete_voice_profile | 删除自定义音色 |
| get_service_status | 获取 TTS 引擎状态 |
| pick_audio_file | 原生文件选择对话框 |
| get_license_status | 获取授权状态 |
| activate_license | 激活授权码 |

### 应用数据目录

```
app-data/
├─ outputs/          # 生成的 WAV 音频
├─ voices/profiles/  # 声音克隆 profile
├─ models/           # Qwen3 TTS 模型文件
├─ logs/
└─ license.json
```

### 构建与部署

```bash
pnpm dev           # Vite 开发服务器 :1420
pnpm tauri:dev     # Tauri 桌面开发模式
make build-linux   # .deb + .rpm + .AppImage
make build-windows # .exe + .msi
make build-mac     # .dmg + .app
make release VERSION=1.0.0  # 推送 tag → 触发 CI 自动构建并上传七牛云
```

## 注意事项

1. **模型文件**：需用户自行下载 Qwen3 TTS 模型放入 `models/`
2. **LibTorch**：编译需安装 libtorch 或设 `LIBTORCH` 环境变量
3. **GPU 加速**：自动检测 CUDA
4. **模型懒加载**：首次 generate_speech 触发加载，耗时较长
5. **项目更新时，本文件需同步更新**
