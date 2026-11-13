#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "==> 1. 建置 WASM"
./scripts/build-wasm.sh

echo "==> 2. 建置前端 (GitHub Pages 模式)"
GITHUB_PAGES=true pnpm -C apps/web build

echo "==> 建置完成！靜態檔案位於 apps/web/dist"
