#!/bin/sh
set -eu

APP_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
WIT_DIR="$APP_DIR/wit"
OUT_DIR="$APP_DIR/target/wasm32-wasip2/release"
OUT_WASM="$OUT_DIR/vellum_markdown_demo.wasm"

cd "$APP_DIR"

moon build --target wasm --release

mkdir -p "$OUT_DIR"

wasm-tools component embed "$WIT_DIR" "_build/wasm/release/build/gen/gen.wasm" \
  --world app-world \
  --encoding utf16 \
  -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
  -o "$OUT_WASM"

echo "Built: $OUT_WASM"
