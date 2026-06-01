#!/usr/bin/env bash
# Build the release wasm. Output: target/wasm32-wasip1/release/zhop.wasm
set -euo pipefail
cd "$(dirname "$0")"

if ! rustup target list --installed 2>/dev/null | grep -q wasm32-wasip1; then
  echo "→ adding wasm32-wasip1 target"
  rustup target add wasm32-wasip1
fi

cargo build --release
WASM="target/wasm32-wasip1/release/zhop.wasm"
echo "✓ built $WASM ($(du -h "$WASM" | cut -f1))"
