#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${1:-8080}"

if [[ ! -f "$ROOT_DIR/website/pkg/vetrace_web.js" || ! -f "$ROOT_DIR/website/pkg/vetrace_web_bg.wasm" ]]; then
  echo "The generated WebAssembly package is missing." >&2
  if command -v rustup >/dev/null 2>&1 && command -v wasm-pack >/dev/null 2>&1; then
    echo "Building it now with ./scripts/build_web.sh ..." >&2
    "$ROOT_DIR/scripts/build_web.sh"
  else
    cat >&2 <<'MESSAGE'
Install the browser build tools, then run:
  rustup target add wasm32-unknown-unknown
  cargo install wasm-pack
  ./scripts/build_web.sh
  ./scripts/serve_web.sh
MESSAGE
    exit 1
  fi
fi

python3 "$ROOT_DIR/scripts/serve_web.py" --directory "$ROOT_DIR" --port "$PORT" --bind 127.0.0.1
