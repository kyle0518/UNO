#!/usr/bin/env bash
# 安裝工具鏈與依賴（可重複執行）
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "==> 安裝 Rust"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

echo "==> Rust 元件"
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "==> 安裝 wasm-pack"
  cargo install wasm-pack --locked
fi

if ! command -v uv >/dev/null 2>&1; then
  echo "==> 安裝 uv"
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="$HOME/.local/bin:$PATH"
fi

if [ -f package.json ]; then
  echo "==> pnpm install"
  pnpm install
fi

if [ -f python/pyproject.toml ]; then
  echo "==> Python venv"
  (cd python && uv venv --python 3.12 .venv && uv pip install -p .venv maturin)
fi

echo "==> 完成"
