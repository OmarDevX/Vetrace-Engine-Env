#!/usr/bin/env python3
"""Validate that every WGPU WGSL shader parses with naga.

This catches broken or stale shaders before they can stall/fail WGPU pipeline
creation at application startup. It expects the `naga` CLI to be available on
PATH (for example: `cargo install naga-cli --version 0.20.0`).
"""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHADER_ROOT = ROOT / "vetrace_engine" / "assets" / "shaders" / "wgpu"


def main() -> int:
    naga = shutil.which("naga")
    if naga is None:
        print(
            "naga CLI not found; install with `cargo install naga-cli --version 0.20.0`",
            file=sys.stderr,
        )
        return 2

    failures: list[tuple[Path, str]] = []
    for shader in sorted(SHADER_ROOT.rglob("*.wgsl")):
        result = subprocess.run(
            [naga, str(shader)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if result.returncode != 0:
            failures.append((shader.relative_to(ROOT), result.stderr or result.stdout))

    if failures:
        for shader, error in failures:
            print(f"{shader}:\n{error}", file=sys.stderr)
        return 1

    print("WGSL syntax validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
