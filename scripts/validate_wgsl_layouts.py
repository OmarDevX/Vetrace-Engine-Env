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
    match = re.search(rf"struct {re.escape(struct_name)}\s*\{{(?P<body>.*?)\}};", source, re.S)
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


def extract_bracketed_block(source: str, start: int) -> tuple[str, int]:
    depth = 0
    block_start = start
    for i in range(start, len(source)):
        ch = source[i]
        if ch == "[":
            if depth == 0:
                block_start = i + 1
            depth += 1
        elif ch == "]":
            depth -= 1
            if depth == 0:
                return source[block_start:i], i + 1
    raise AssertionError("unterminated bracketed block")


def rust_compute_bind_group_descriptor_bindings() -> list[set[int]]:
    source = (
        ROOT / "vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs"
    ).read_text()
    descriptors: list[set[int]] = []
    search_from = 0
    label = 'label: Some("compute_bg")'
    while True:
        start = source.find(label, search_from)
        if start < 0:
            break
        entries_start = source.find("entries: &[", start)
        if entries_start < 0:
            raise AssertionError("compute_bg entries not found")
        descriptor_source, search_from = extract_bracketed_block(
            source, source.find("[", entries_start)
        )
        descriptors.append(
            {int(binding) for binding in re.findall(r"binding:\s*(\d+)", descriptor_source)}
        )
    if not descriptors:
        raise AssertionError("compute_bg descriptors not found")
    return descriptors

def assert_compute_bind_groups_match_layout() -> None:
    layout_bindings = rust_compute_bind_group_bindings()
    descriptors = rust_compute_bind_group_descriptor_bindings()
    for index, descriptor_bindings in enumerate(descriptors, 1):
        missing = sorted(layout_bindings - descriptor_bindings)
        extra = sorted(descriptor_bindings - layout_bindings)
        if missing or extra:
            raise AssertionError(
                f"compute_bg descriptor {index} does not match compute_bgl; "
                f"missing={missing}, extra={extra}"
            )

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


DDGI_SHADERS = [
    "vetrace_engine/assets/shaders/wgpu/hybrid/ddgi_trace.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/ddgi_classify.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/ddgi_relocate.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/ddgi_update_irradiance.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/ddgi_update_distance.comp.wgsl",
    "vetrace_engine/assets/shaders/wgpu/hybrid/ddgi_fixup_borders.comp.wgsl",
]


def assert_ddgi_contract() -> None:
    types_source = (ROOT / "vetrace_engine/src/rendering/wgpu_renderer/types.rs").read_text()
    components_source = (ROOT / "vetrace_engine/src/components/components.rs").read_text()
    renderer_source = (ROOT / "vetrace_engine/src/rendering/renderer.rs").read_text()
    gi_resolve_source = (ROOT / "vetrace_engine/assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl").read_text()

    required_constants = [
        "pub const GI_MODE_DDGI: u32 = 6;",
        "pub const GI_RESOLVE_METHOD_DDGI: u32 = GI_MODE_DDGI;",
        "pub const DDGI_DEBUG_VIEW_IRRADIANCE: u32 = 1;",
        "pub const DDGI_DEBUG_VIEW_DEPTH_VISIBILITY: u32 = 2;",
        "pub const DDGI_DEBUG_VIEW_PROBE_STATE: u32 = 3;",
        "pub const DDGI_DEBUG_VIEW_RELOCATION_OFFSETS: u32 = 4;",
        "pub const DDGI_DEBUG_VIEW_VOLUME_COORDINATES: u32 = 5;",
        "pub const DDGI_DEBUG_VIEW_ACTIVE_FALLBACK_STATUS: u32 = 6;",
        "pub const DDGI_DEBUG_VIEW_PROBE_TILE_INDEX: u32 = 7;",
    ]
    missing = [constant for constant in required_constants if constant not in types_source]
    if missing:
        raise AssertionError(f"DDGI Rust constants missing/drifted: {missing}")

    if "DDGI = 6" not in components_source:
        raise AssertionError("GlobalIlluminationMode::DDGI must remain ABI value 6")
    if "pub ddgi_debug_view: u32" not in renderer_source or "pub ddgi_debug_view: u32" not in components_source:
        raise AssertionError("DDGI debug view must be routed through PostProcessing and RenderParams")
    if "const GI_RESOLVE_METHOD_DDGI: u32 = 6u;" not in gi_resolve_source:
        raise AssertionError("gi_resolve DDGI method constant drifted from Rust")
    if "@group(0) @binding(14) var<storage, read> ddgi_probe_state: array<u32>;" not in gi_resolve_source:
        raise AssertionError("gi_resolve must read DDGI probe state as array<u32>, matching trace/classify")
    if "let ddgi_debug_view = (params.debug_flags >> 8u) & 0xffu;" not in gi_resolve_source:
        raise AssertionError("gi_resolve must unpack DDGI debug view from debug_flags bits 8..15")

    rust_gi_resolve_fields = rust_struct_fields(
        "vetrace_engine/src/rendering/wgpu_renderer/types.rs", "GiResolveParams"
    )
    wgsl_gi_resolve_fields, _ = wgsl_struct_fields(
        "vetrace_engine/assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl", "GiResolveParams"
    )
    if rust_gi_resolve_fields != wgsl_gi_resolve_fields:
        raise AssertionError(
            "GiResolveParams Rust/WGSL field order mismatch\n"
            f"expected: {rust_gi_resolve_fields}\nactual:   {wgsl_gi_resolve_fields}"
        )

    rust_ddgi_fields = rust_struct_fields(
        "vetrace_engine/src/rendering/wgpu_renderer/types.rs", "DdgiTraceUpdateUniforms"
    )
    for path in DDGI_SHADERS:
        wgsl_fields, source = wgsl_struct_fields(path, "DdgiParams")
        if rust_ddgi_fields != wgsl_fields:
            raise AssertionError(
                f"{path} DdgiParams mismatch\nexpected: {rust_ddgi_fields}\nactual:   {wgsl_fields}"
            )
        if "probe_states: array<u32>" in source and "probe_states[" not in source:
            raise AssertionError(f"{path} declares probe_states but does not use it")

def main() -> int:
    legacy_renderer = ROOT / "vetrace_engine/src/rendering/wgpu_renderer/types.rs"
    if legacy_renderer.exists():
        assert_shader_params_prefixes()
        assert_material_stride_contract()
        assert_gbuffer_contract()
        assert_no_runtime_indexed_inline_arrays()
        assert_compute_bind_groups_match_layout()
        assert_hybrid_compose_compute_layout_matches_shader()
        assert_ddgi_contract()
    else:
        print("legacy monolithic WGPU layout checks skipped; renderer sources were removed")

    # The active renderer owns its own Rust/WGSL ABI and bind-group routing.
    from validate_environment_cubemap import main as validate_environment_cubemap

    validate_environment_cubemap()
    print("WGSL layout validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(exc, file=sys.stderr)
        raise SystemExit(1)
