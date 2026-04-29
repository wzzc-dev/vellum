#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

WIT_DIR="$SCRIPT_DIR/wit"

wit-bindgen moonbit "$WIT_DIR/vellum-extension.wit" \
    --world extension-world \
    --out-dir "$SCRIPT_DIR" \
    --derive-show --derive-eq --derive-error \
    --ignore-stub

find . -name "moon.pkg.json" -not -path "./_build/*" -exec sed -i '' 's|vellum/extension/interface/vellum/extension/|vellum/pomodoro/interface/vellum/extension/|g' {} +

moon build --target wasm --release

WASM_INPUT="_build/wasm/release/build/gen/gen.wasm"

mkdir -p target/wasm32-wasip2/release

wasm-tools component embed "$WIT_DIR" "$WASM_INPUT" \
    --world extension-world \
    --encoding utf16 \
    -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
    -o target/wasm32-wasip2/release/vellum_pomodoro.wasm

echo "Built: target/wasm32-wasip2/release/vellum_pomodoro.wasm"
