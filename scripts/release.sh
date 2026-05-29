#!/usr/bin/env bash
set -euo pipefail

# 一键发版：推分支 + 打 tag + 推 tag → 触发 Gitee Go 流水线
# Usage:
#   scripts/release.sh 1.0.1

VERSION="${1:-}"
REMOTE="${2:-origin}"

if [[ -z "$VERSION" ]]; then
  echo "Usage: scripts/release.sh <version> [remote]"
  exit 1
fi

TAG="v${VERSION#v}"

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "Error: not inside a git repository"
  exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree has uncommitted changes. Commit or stash before release."
  exit 1
fi

if ! git remote get-url "$REMOTE" >/dev/null 2>&1; then
  echo "Error: remote '$REMOTE' not found"
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Error: tag '$TAG' already exists locally"
  exit 1
fi

echo "[1/3] Push current branch to $REMOTE"
CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
git push "$REMOTE" "$CURRENT_BRANCH"

echo "[2/3] Create tag $TAG"
git tag "$TAG"

echo "[3/3] Push tag to $REMOTE"
git push "$REMOTE" "$TAG"

echo "Release trigger completed for tag $TAG"
echo "Gitee Go 流水线将自动构建 Linux 安装包并上传至："
echo "  https://gitee.com/imglingdu/pc_clinet/releases"
