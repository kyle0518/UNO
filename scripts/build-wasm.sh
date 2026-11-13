#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"

mkdir -p "$ROOT/apps/web/src/wasm/pkg"
echo "==> 建置 WASM"
wasm-pack build crates/uno-wasm --target web --out-dir "$ROOT/apps/web/src/wasm/pkg" --release
