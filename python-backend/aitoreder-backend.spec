# -*- mode: python ; coding: utf-8 -*-
# PyInstaller spec for aitoreder-backend sidecar
#
# 使用方法：
#   cd python-backend
#   python -m PyInstaller aitoreder-backend.spec --clean
#
# 构建完成后将 dist/aitoreder-backend/ 整个目录
# 复制到 src-tauri/binaries/aitoreder-backend/，
# 或者直接运行：make build-sidecar

from PyInstaller.utils.hooks import (
    collect_data_files,
    collect_dynamic_libs,
)

block_cipher = None

# --- 收集各包附带的非 Python 文件（模型配置、字体、.so 等） ---
added_datas = []
added_datas += collect_data_files("torch", include_py_files=False)
added_datas += collect_data_files("torchaudio", include_py_files=False)
added_datas += collect_data_files("TTS", include_py_files=False)
added_datas += collect_data_files("librosa", include_py_files=False)

added_binaries = []
added_binaries += collect_dynamic_libs("torch")
added_binaries += collect_dynamic_libs("torchaudio")
added_binaries += collect_dynamic_libs("soundfile")

a = Analysis(
    ["app.py"],
    pathex=[],
    binaries=added_binaries,
    datas=added_datas,
    hiddenimports=[
        # uvicorn 内部模块（import 字符串延迟加载，PyInstaller 检测不到）
        "uvicorn.logging",
        "uvicorn.loops",
        "uvicorn.loops.auto",
        "uvicorn.loops.asyncio",
        "uvicorn.protocols",
        "uvicorn.protocols.http",
        "uvicorn.protocols.http.auto",
        "uvicorn.protocols.http.h11_impl",
        "uvicorn.protocols.websockets",
        "uvicorn.protocols.websockets.auto",
        "uvicorn.lifespan",
        "uvicorn.lifespan.off",
        "uvicorn.lifespan.on",
        # starlette / anyio
        "anyio._backends._asyncio",
        "starlette.routing",
        "starlette.staticfiles",
        # pydantic v2
        "pydantic.deprecated.class_validators",
        # 音频 I/O
        "soundfile",
        # 本项目 services 包
        "services.tts_service",
        "services.voice_clone_service",
        "services.audio_utils",
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    # 排除开发/调试工具，减小体积
    excludes=["tkinter", "matplotlib", "IPython", "notebook", "pytest", "PIL"],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="aitoreder-backend",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="aitoreder-backend",
)
