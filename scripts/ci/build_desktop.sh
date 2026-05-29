#!/usr/bin/env bash
set -euo pipefail

# Unified desktop packaging script for CI runners.
# Usage:
#   scripts/ci/build_desktop.sh nsis,msi
#   scripts/ci/build_desktop.sh dmg,app

BUNDLES="${1:-all}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cd "$ROOT_DIR"

echo "[1/3] Install frontend dependencies"
pnpm install --frozen-lockfile

echo "[2/3] Build Tauri bundles: ${BUNDLES}"
pnpm exec tauri build --bundles "$BUNDLES"

echo "Done. Bundles are in src-tauri/target/release/bundle"
