#!/usr/bin/env bash
# 一鍵啟動（PLAN T11.5）：build-wasm → gen-types → web build → uno-server --release
# 用法：./scripts/start.sh [--port 8787] [--host 0.0.0.0] [其他 uno-server 參數...]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"

./scripts/build-wasm.sh
./scripts/gen-types.sh
pnpm -C apps/web build
exec cargo run -p uno-server --release -- "$@"
