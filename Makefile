PNPM ?= pnpm

.DEFAULT_GOAL := help

.PHONY: help install install-frontend dev build tauri-dev tauri-build build-windows build-linux build-mac release clean

help:
	@echo "可用命令:"
	@echo "  make install            安装前端依赖"
	@echo "  make install-frontend   安装前端依赖"
	@echo "  make dev                启动前端开发服务 (Vite)"
	@echo "  make build              构建前端产物"
	@echo "  make tauri-dev          启动 Tauri 开发模式"
	@echo "  make tauri-build        构建 Tauri 桌面应用（当前平台）"
	@echo "  make build-windows      打包 Windows 安装包 (NSIS .exe + .msi)"
	@echo "  make build-linux        打包 Linux 安装包 (.deb + .rpm + .AppImage)"
	@echo "  make build-mac          打包 macOS 安装包 (.dmg + .app)"
	@echo "  make release VERSION=1.0.0  一键发版：推送分支+打 tag+推送 tag"
	@echo "  make clean              清理前端构建产物"

install: install-frontend

install-frontend:
	$(PNPM) install

dev:
	$(PNPM) dev

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

release:
	@if [ -z "$(VERSION)" ]; then \
		echo "请提供版本号，例如: make release VERSION=1.0.0"; \
		exit 1; \
	fi
	bash scripts/release.sh "$(VERSION)"

clean:
	node -e "require('fs').rmSync('dist', { recursive: true, force: true })"
