#!/bin/bash
# Quick build script for Vellum Pomodoro extension
# Faster than full build for hot-reload scenarios

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Skip WIT binding generation for speed
# Build the MoonBit package
moon build --target wasm --release

WASM_INPUT="_build/wasm/release/build/gen/gen.wasm"
mkdir -p target/wasm32-wasip2/release

# Fast component embedding and creation
wasm-tools component embed wit "$WASM_INPUT" \
    --world extension-world \
    --encoding utf16 \
    -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
    -o target/wasm32-wasip2/release/vellum_pomodoro.wasm

echo "✅ Quick build completed: target/wasm32-wasip2/release/vellum_pomodoro.wasm"
