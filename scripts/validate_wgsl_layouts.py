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

GBUFFER_CONTRACT_SHADERS = [
    "vetrace_engine/assets/shaders/wgpu/hybrid/primitive_gbuffer.wgsl",
    "vetrace_engine/shaders/simple_pbr.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl",
]

GBUFFER_CONTRACT_SNIPPETS = [
    "gbuf_albedo rgba8unorm: rgb = linear base color, a = coverage/valid surface mask",
    "gbuf_normal rgba16float: xyz = world-space normal encoded",
    "gbuf_material rgba8uint: x = metallic UNORM8, y = roughness UNORM8, z = emissive luma UNORM8",
    "low nibble = feature flags, high nibble = object/material ID bucket",
    "depth texture r32float: device depth",
    "const GBUFFER_FEATURE_FLAGS_MASK: u32 = 0x0fu",
    "const GBUFFER_ID_SHIFT: u32 = 4u",
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



def assert_gbuffer_contract() -> None:
    for path in GBUFFER_CONTRACT_SHADERS:
        source = (ROOT / path).read_text()
        missing = [snippet for snippet in GBUFFER_CONTRACT_SNIPPETS if snippet not in source]
        if missing:
            raise AssertionError(f"{path} missing G-buffer contract snippets: {missing}")

    for path in GBUFFER_CONTRACT_SHADERS[:2]:
        source = (ROOT / path).read_text()
        required = [
            "encode_gbuffer_unorm8",
            "encode_gbuffer_metadata",
            "encode_gbuffer_unorm8(m",
        ] if path.endswith("simple_pbr.wgsl") else [
            "encode_gbuffer_unorm8(mat.metallicFactor)",
            "encode_gbuffer_unorm8(mat.roughnessFactor)",
            "encode_gbuffer_unorm8(emissive_luma)",
            "encode_gbuffer_metadata(id_bucket, feature_flags)",
        ]
        missing = [snippet for snippet in required if snippet not in source]
        if missing:
            raise AssertionError(f"{path} does not encode material channels through the shared G-buffer helpers: {missing}")

    compose = (ROOT / "vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl").read_text()
    if "decode_gbuffer_material(textureLoad(gbuf_material" not in compose:
        raise AssertionError("hybrid_compose.comp.wgsl must consume gbuf_material through decode_gbuffer_material")
    forbidden = ["f32(material.x) / 255.0", "f32(material.y) / 255.0", "f32(material.z) / 255.0"]
    found = [snippet for snippet in forbidden if snippet in compose]
    if found:
        raise AssertionError(f"hybrid_compose.comp.wgsl still has pass-specific material channel assumptions: {found}")


def assert_no_runtime_indexed_inline_arrays() -> None:
    pattern = re.compile(r"array<[^\n]+>\([^\n]+\)\[[A-Za-z_]\w*\]")
    for shader in (ROOT / "vetrace_engine/assets/shaders/wgpu").rglob("*.wgsl"):
        path = shader.relative_to(ROOT).as_posix()
        source = shader.read_text()
        match = pattern.search(source)
        if match:
            raise AssertionError(
                f"{path} uses an inline array indexed by a runtime value: {match.group(0)}"
            )

def shader_bindings(source: str) -> set[tuple[int, int]]:
    return {
        (int(group), int(binding))
        for group, binding in re.findall(
            r"@group\((\d+)\)\s*@binding\((\d+)\)", source
        )
    }


def rust_compute_bind_group_bindings() -> set[int]:
    source = (
        ROOT / "vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs"
    ).read_text()
    label = 'label: Some("compute_bgl")'
    start = source.find(label)
    if start < 0:
        raise AssertionError("compute_bgl layout not found")
    end = source.find(
        'boot_log("WgpuRenderer::new: after compute bind group layout")', start
    )
    if end < 0:
        raise AssertionError("compute_bgl layout end marker not found")
    layout_source = source[start:end]
    return {int(binding) for binding in re.findall(r"binding:\s*(\d+)", layout_source)}


def assert_hybrid_compose_compute_layout_matches_shader() -> None:
    source = (
        (
            ROOT / "vetrace_engine/assets/shaders/wgpu/hybrid/pbr_lighting.wgsl"
        ).read_text()
        + "\n"
        + (
            ROOT / "vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl"
        ).read_text()
    )
    required_bindings = {binding for group, binding in shader_bindings(source) if group == 0}
    layout_bindings = rust_compute_bind_group_bindings()
    missing = sorted(required_bindings - layout_bindings)
    if missing:
        raise AssertionError(
            f"hybrid_compose_pipeline compute_bgl is missing shader bindings: {missing}"
        )

def main() -> int:
    assert_shader_params_prefixes()
    assert_material_stride_contract()
    assert_gbuffer_contract()
    assert_no_runtime_indexed_inline_arrays()
    assert_hybrid_compose_compute_layout_matches_shader()
    print("WGSL layout validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(exc, file=sys.stderr)
        raise SystemExit(1)
