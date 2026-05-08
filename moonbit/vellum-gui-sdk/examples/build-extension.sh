#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: build-extension.sh EXTENSION_DIR SLUG" >&2
  exit 2
fi

EXTENSION_DIR="$1"
SLUG="$2"
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

cd "$EXTENSION_DIR"

WIT_DIR="$EXTENSION_DIR/wit"
OUT_SLUG="$(printf '%s' "$SLUG" | tr '-' '_')"
OUT_DIR="target/wasm32-wasip2/release"
OUT_WASM="$OUT_DIR/vellum_sdk_example_$OUT_SLUG.wasm"

moon build --target wasm --release

WASM_INPUT="_build/wasm/release/build/gen/gen.wasm"
mkdir -p "$OUT_DIR"

wasm-tools component embed "$WIT_DIR" "$WASM_INPUT" \
  --world extension-world \
  --encoding utf16 \
  -o target/component.embed.wasm

wasm-tools component new target/component.embed.wasm \
  -o "$OUT_WASM"

echo "Built: $EXTENSION_DIR/$OUT_WASM"

