#!/bin/sh
set -eu

APP_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$APP_DIR/../../.." && pwd)"
WIT_FILE="$REPO_ROOT/wit/vellum-app.wit"
BUILD_WASM="$REPO_ROOT/_build/wasm/release/build/vellum/demo-markdown-editor/gen/gen.wasm"
OUT_DIR="$APP_DIR/target/wasm32-wasip2/release"
OUT_WASM="$OUT_DIR/vellum_markdown_demo.wasm"

cd "$APP_DIR"

moon build --target wasm --release --target-dir "$REPO_ROOT/_build"

mkdir -p "$OUT_DIR"

wasm-tools component embed "$WIT_FILE" "$BUILD_WASM" \
  --world app-world \
  --encoding utf16 \
  -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
  -o "$OUT_WASM"

echo "Built: $OUT_WASM"
