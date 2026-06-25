#!/usr/bin/env python3
"""Validate Rust/WGSL GPU layout contracts that are easy to drift.

This intentionally uses only the Python standard library so it can run even in
minimal CI environments where Rust graphics system packages are unavailable.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

PARAMS_SHADERS = [
    "vetrace_engine/assets/shaders/wgpu/hybrid/raytrace.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/denoise.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/rt_denoise.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/sdfgi_prepass.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/sdfgi_inject.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/atmosphere/transmittance_lut.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/atmosphere/sky_view_lut.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/atmosphere/multi_scattering_lut.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/atmosphere/aerial_perspective_lut.comp.wgsl",
]

MATERIAL_SHADERS = [
    "vetrace_engine/assets/shaders/wgpu/hybrid/raytrace.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl",
]

EXPECTED_MATERIAL_TAIL = [
    "custom_material_id",
    "material_flags0",
    "material_flags1",
    "material_flags2",
    "material_flags3",
    "material_flags4",
    "material_flags5",
    "material_flags6",
]


def rust_struct_fields(path: str, struct_name: str) -> list[str]:
    source = (ROOT / path).read_text()
    match = re.search(rf"pub struct {re.escape(struct_name)}\s*\{{(?P<body>.*?)\n\}}", source, re.S)
    if not match:
        raise AssertionError(f"{struct_name} not found in {path}")
    fields: list[str] = []
    for line in match.group("body").splitlines():
        line = line.split("//", 1)[0].strip()
        if line.startswith("pub "):
            fields.append(line.split(":", 1)[0].removeprefix("pub ").strip())
    return fields


def wgsl_struct_fields(path: str, struct_name: str) -> tuple[list[str], str]:
    source = (ROOT / path).read_text()
    match = re.search(rf"struct {re.escape(struct_name)}\s*\{{(?P<body>.*?)\n\}};", source, re.S)
    if not match:
        raise AssertionError(f"{struct_name} not found in {path}")
    fields: list[str] = []
    for line in match.group("body").splitlines():
        line = line.split("//", 1)[0]
        for chunk in line.replace(";", ",").split(","):
            if ":" in chunk:
                fields.append(chunk.split(":", 1)[0].strip())
    return fields, source


def assert_shader_params_prefixes() -> None:
    rust_fields = rust_struct_fields(
        "vetrace_engine/src/rendering/wgpu_renderer/types.rs", "ShaderParams"
    )
    for path in PARAMS_SHADERS:
        wgsl_fields, _ = wgsl_struct_fields(path, "Params")
        expected = rust_fields[: len(wgsl_fields)]
        if wgsl_fields != expected:
            raise AssertionError(
                f"{path} Params mismatch\nexpected: {expected}\nactual:   {wgsl_fields}"
            )


def assert_material_stride_contract() -> None:
    rust_fields = rust_struct_fields("vetrace_engine/src/scene/object.rs", "GpuMaterial")
    if rust_fields[-2:] != ["custom_material_id", "_pad2"]:
        raise AssertionError(f"GpuMaterial trailing fields changed: {rust_fields[-2:]}")

    for path in MATERIAL_SHADERS:
        wgsl_fields, source = wgsl_struct_fields(path, "MaterialParams")
        tail = wgsl_fields[-len(EXPECTED_MATERIAL_TAIL) :]
        if tail != EXPECTED_MATERIAL_TAIL:
            raise AssertionError(
                f"{path} MaterialParams tail must be {EXPECTED_MATERIAL_TAIL}, got {tail}"
            )
        if "_pad2: vec3<u32>" in source or "mat._pad2" in source:
            raise AssertionError(f"{path} still uses old vec3 padding/material flag access")


def main() -> int:
    assert_shader_params_prefixes()
    assert_material_stride_contract()
    print("WGSL layout validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(exc, file=sys.stderr)
        raise SystemExit(1)
