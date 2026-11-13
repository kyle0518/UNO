#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> cargo fmt --check"
cargo fmt --all --check

echo "==> cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo test"
cargo test --workspace

if [ -d apps/web ]; then
  echo "==> web lint / typecheck / test"
  pnpm -C apps/web lint
  pnpm -C apps/web typecheck
  pnpm -C apps/web test
fi

if [ -x python/.venv/bin/pytest ] && [ -d python/tests ]; then
  echo "==> pytest"
  python/.venv/bin/pytest python/tests
fi

echo "==> 全部檢查通過"
