#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

"$SCRIPT_DIR/markdown-demo/build.sh"
