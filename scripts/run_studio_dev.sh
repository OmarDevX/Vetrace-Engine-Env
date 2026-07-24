#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
cargo build -p vetrace_player -p vetrace_studio
export VETRACE_PLAYER="$ROOT/target/debug/vetrace-player"
exec "$ROOT/target/debug/vetrace-studio" "$@"
