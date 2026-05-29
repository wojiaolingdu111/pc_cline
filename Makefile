PNPM ?= pnpm
PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
VENV_DIR ?= .venv
VENV_PYTHON ?= $(VENV_DIR)/bin/python
VENV_PIP ?= $(VENV_PYTHON) -m pip

.DEFAULT_GOAL := help

.PHONY: help install setup-venv install-frontend install-backend backend-dev dev dev-all build tauri-dev tauri-build build-windows build-linux build-mac build-sidecar release clean

help:
	@echo "可用命令:"
	@echo "  make install            安装前后端依赖"
	@echo "  make setup-venv         创建/更新项目 Python 虚拟环境 (.venv)"
	@echo "  make install-frontend   安装前端依赖"
	@echo "  make install-backend    安装 Python 依赖"
	@echo "  make backend-dev        启动 Python 后端 (127.0.0.1:8765)"
	@echo "  make dev                启动前端开发服务 (Vite)"
	@echo "  make dev-all            同时启动 Python 后端 + 前端开发服务"
	@echo "  make build              构建前端产物"
	@echo "  make tauri-dev          启动 Tauri 开发模式"
	@echo "  make tauri-build        构建 Tauri 桌面应用（当前平台）"
	@echo "  make build-windows      打包 Windows 安装包 (NSIS .exe + .msi)"
	@echo "  make build-linux        打包 Linux 安装包 (.deb + .rpm + .AppImage)"
	@echo "  make build-mac          打包 macOS 安装包 (.dmg + .app)"
	@echo "  make build-sidecar      用 PyInstaller 编译 Python 后端为第一方 sidecar"
	@echo "                          (构建安装包前必须先运行此命令！)"
	@echo "  make release VERSION=1.0.0  一键发版：推送分支+打 tag+推送 tag"
	@echo "  make clean              清理前端构建产物"

install: install-frontend install-backend

setup-venv:
	$(PYTHON) -m venv $(VENV_DIR)
	$(VENV_PIP) install --upgrade pip

install-frontend:
	$(PNPM) install

install-backend: setup-venv
	cd python-backend && ../$(VENV_PIP) install -r requirements.txt

backend-dev: install-backend
	cd python-backend && ../$(VENV_PYTHON) app.py

dev:
	$(PNPM) dev

dev-all:
	$(PNPM) dlx concurrently --names backend,frontend --prefix-colors cyan,green "cd python-backend && $(PYTHON) app.py" "$(PNPM) dev"

build:
	$(PNPM) build

tauri-dev:
	$(PNPM) tauri:dev

tauri-build:
	$(PNPM) tauri:build

build-windows:
	@if [ "$(OS)" != "Windows_NT" ]; then \
		echo "当前主机不是 Windows，无法本地打包 Windows 安装包。"; \
		echo "请使用 GitHub Actions（推送 v* 标签自动构建）。"; \
		exit 1; \
	fi
	$(PNPM) exec tauri build --bundles nsis,msi

build-linux:
	$(PNPM) exec tauri build --bundles deb,rpm,appimage

build-mac:
	@if [ "$$(uname -s)" != "Darwin" ]; then \
		echo "当前主机不是 macOS，无法本地打包 macOS 安装包。"; \
		echo "请使用 GitHub Actions（推送 v* 标签自动构建）。"; \
		exit 1; \
	fi
	$(PNPM) exec tauri build --bundles dmg,app

# 将 Python 后端编译为独立可执行文件（sidecar）
# 自动在项目虚拟环境中安装并使用 PyInstaller
# 构建后自动将产物拷贝到 src-tauri/binaries/aitoreder-backend/
build-sidecar: setup-venv
	cd python-backend && ../$(VENV_PIP) install pyinstaller && ../$(VENV_PYTHON) -m PyInstaller aitoreder-backend.spec --clean --distpath ../src-tauri/binaries
	@test -d src-tauri/binaries/aitoreder-backend || (echo "sidecar 构建失败：未生成 src-tauri/binaries/aitoreder-backend" && exit 1)

release:
	@if [ -z "$(VERSION)" ]; then \
		echo "请提供版本号，例如: make release VERSION=1.0.0"; \
		exit 1; \
	fi
	bash scripts/release.sh "$(VERSION)"

clean:
	node -e "require('fs').rmSync('dist', { recursive: true, force: true })"
