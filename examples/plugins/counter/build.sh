#!/bin/sh
set -eu

PLUGIN_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$PLUGIN_DIR/../../.." && pwd)"
WIT_FILE="$REPO_ROOT/wit/vellum-app.wit"
BUILD_WASM="$REPO_ROOT/_build/wasm/release/build/vellum/plugin-counter/gen/gen.wasm"
OUT_DIR="$PLUGIN_DIR/target/wasm32-wasip2/release"
OUT_WASM="$OUT_DIR/vellum_counter_plugin.wasm"

cd "$PLUGIN_DIR"

moon build --target wasm --release --target-dir "$REPO_ROOT/_build"

mkdir -p "$OUT_DIR"

wasm-tools component embed "$WIT_FILE" "$BUILD_WASM" \
  --world app-world \
  --encoding utf16 \
  -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
  -o "$OUT_WASM"

echo "Built: $OUT_WASM"
