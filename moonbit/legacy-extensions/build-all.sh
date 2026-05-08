#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

for slug in counter form todo-list timer navigation gesture transition comprehensive wit-counter; do
  "$SCRIPT_DIR/$slug/build.sh"
done

echo "All MoonBit SDK example extensions built."

