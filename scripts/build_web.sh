#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v rustup >/dev/null 2>&1; then
  echo "error: rustup is required to install the wasm32 target" >&2
  exit 1
fi
if ! command -v wasm-pack >/dev/null 2>&1; then
  cat >&2 <<'MESSAGE'
error: wasm-pack is required.
Install it with:
  cargo install wasm-pack
MESSAGE
  exit 1
fi

python3 "$ROOT_DIR/scripts/validate_web_port.py"
rustup target add wasm32-unknown-unknown
rm -rf "$ROOT_DIR/website/pkg"
mkdir -p "$ROOT_DIR/website/pkg"

(
  cd "$ROOT_DIR/vetrace_web"
  wasm-pack build \
    --target web \
    --release \
    --out-dir ../website/pkg \
    --out-name vetrace_web
)

if [[ ! -f "$ROOT_DIR/website/pkg/vetrace_web.js" || ! -f "$ROOT_DIR/website/pkg/vetrace_web_bg.wasm" ]]; then
  echo "error: wasm-pack completed without the expected browser package" >&2
  exit 1
fi

if ! grep -q "start_example" "$ROOT_DIR/website/pkg/vetrace_web.js"; then
  echo "error: generated JavaScript package does not export start_example" >&2
  exit 1
fi

python3 - <<PY_BUILD_INFO
import json
from datetime import datetime, timezone
from pathlib import Path
path = Path(r"$ROOT_DIR/website/pkg/build.json")
path.write_text(json.dumps({
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "package": "vetrace_web",
    "loader": "v5",
}, indent=2) + "\n", encoding="utf-8")
PY_BUILD_INFO

touch "$ROOT_DIR/website/pkg/.gitkeep"
echo "Built Vetrace Web into website/pkg"
