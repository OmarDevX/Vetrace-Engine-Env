#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
for file in vetrace_web.js vetrace_web_bg.wasm; do
  path="$ROOT_DIR/website/pkg/$file"
  if [[ ! -s "$path" ]]; then
    echo "missing: website/pkg/$file" >&2
    exit 1
  fi
done
if ! grep -q "start_example" "$ROOT_DIR/website/pkg/vetrace_web.js"; then
  echo "website/pkg/vetrace_web.js does not export start_example" >&2
  exit 1
fi
echo "Vetrace web package is present and structurally valid."
