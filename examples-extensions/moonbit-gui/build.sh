#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

WIT_DIR="$SCRIPT_DIR/wit"

# Use existing generated files, skip regenerating
if [ ! -d "gen" ]; then
    wit-bindgen moonbit "$WIT_DIR/vellum-extension.wit" \
        --world extension-world \
        --out-dir "$SCRIPT_DIR" \
        --derive-show --derive-eq --derive-error \
        --ignore-stub

    # Fix paths - use Linux style sed
    find . -name "moon.pkg.json" -not -path "./_build/*" -exec sed -i.bak 's|vellum/extension/interface/vellum/extension/|vellum/pomodoro/interface/vellum/extension/|g' {} +
    find . -name "*.bak" -delete
fi

# Fix the import path in gen/moon.pkg.json
# We need it to use relative path instead of absolute package path
if [ -f "gen/moon.pkg.json" ]; then
    cat > gen/moon.pkg.json << 'EOF'
{
      "warn-list": "-44",
      "import": [
      { "path" : "./world/extensionWorld", "alias" : "extensionWorld" }
      ],
      "link": {
            "wasm": {
                  "export-memory-name": "memory",
                  "heap-start-address": 16,
                  "exports": [
                  "mbt_ffi_cabi_realloc:cabi_realloc",
                  "wasmExportActivate:activate",
                  "wasmExportActivatePostReturn:cabi_post_activate",
                  "wasmExportDeactivate:deactivate",
                  "wasmExportDeactivatePostReturn:cabi_post_deactivate",
                  "wasmExportExecuteCommand:execute-command",
                  "wasmExportExecuteCommandPostReturn:cabi_post_execute-command",
                  "wasmExportHandleEvent:handle-event",
                  "wasmExportHandleEventPostReturn:cabi_post_handle-event",
                  "wasmExportHandleHover:handle-hover",
                  "wasmExportHandleHoverPostReturn:cabi_post_handle-hover",
                  "wasmExportHandleUiEvent:handle-ui-event",
                  "wasmExportHandleUiEventPostReturn:cabi_post_handle-ui-event"
                  ]
            }
      }

}
EOF
fi

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
