#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

WIT_DIR="$SCRIPT_DIR/wit"

# Skip WIT binding generation for faster build
# Just build and componentize
moon build --target wasm --release

WASM_INPUT="_build/wasm/release/build/gen/gen.wasm"

mkdir -p target/wasm32-wasip2/release

# Fast component embedding
wasm-tools component embed "$WIT_DIR" "$WASM_INPUT" \
    --world extension-world \
    --encoding utf16 \
    -o target/component.embed.wasm

# Fast component creation
wasm-tools component new target/component.embed.wasm \
    -o target/wasm32-wasip2/release/vellum_pomodoro.wasm

echo "Quick build complete: target/wasm32-wasip2/release/vellum_pomodoro.wasm"
