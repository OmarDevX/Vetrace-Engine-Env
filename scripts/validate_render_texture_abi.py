#!/usr/bin/env python3
"""Validate the active WGPU material/render-texture binding contract.

Uses only the Python standard library, so it can run before Rust/WGPU system
packages are installed.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def rust_bindings(path: Path) -> set[int]:
    source = path.read_text()
    explicit = {int(value) for value in re.findall(r"binding:\s*(\d+)", source)}
    helper_entries = {
        int(value)
        for value in re.findall(r"material_texture_layout_entry\((\d+)\)", source)
    }
    return explicit | helper_entries


def wgsl_bindings(path: Path, group: int) -> set[int]:
    source = path.read_text()
    return {
        int(binding)
        for found_group, binding in re.findall(
            r"@group\((\d+)\)\s*@binding\((\d+)\)", source
        )
        if int(found_group) == group
    }


def main() -> int:
    expected_material = set(range(16))
    layout = rust_bindings(ROOT / "vetrace_render/src/wgpu_window/init_layouts.rs")
    draw = rust_bindings(ROOT / "vetrace_render/src/wgpu_window/draw_cache.rs")
    wgsl = wgsl_bindings(
        ROOT / "vetrace_render/src/wgpu_window/default_material_bindings.wgsl", 0
    )

    # These files contain other bind groups/functions, so compare only the
    # expected active material range instead of requiring exact global equality.
    for label, bindings in [("material layout", layout), ("draw bind group", draw), ("default WGSL", wgsl)]:
        missing = sorted(expected_material - bindings)
        if missing:
            raise AssertionError(f"{label} is missing bindings {missing}")

    portal_source_path = ROOT / "vetrace_render/examples/render_texture_portal.wgsl"
    portal_source = portal_source_path.read_text()
    portal_fragment_block = portal_source.split("struct FragmentInput", 1)[1].split("};", 1)[0]
    portal_fragment_locations = {
        int(value) for value in re.findall(r"@location\((\d+)\)", portal_fragment_block)
    }
    if portal_fragment_locations != {0, 1, 2}:
        raise AssertionError(
            f"portal fragment inputs must be exactly locations 0..2, got {sorted(portal_fragment_locations)}"
        )

    portal_shader = wgsl_bindings(
        portal_source_path, 0
    )
    required_portal = {0, 2, 11}
    missing_portal = sorted(required_portal - portal_shader)
    if missing_portal:
        raise AssertionError(
            f"render_texture_portal.wgsl is missing bindings {missing_portal}"
        )

    custom_source = (
        ROOT / "vetrace_render/src/component_custom_shader.rs"
    ).read_text()
    if "pub render_textures: Vec<String>" not in custom_source:
        raise AssertionError("CustomShaderMaterial render_textures field is missing")
    if "Textured," not in custom_source:
        raise AssertionError("Textured custom-shader vertex interface is missing")

    textured_block = (
        ROOT / "vetrace_render/src/wgpu_window/object_vertex_textured.wgsl"
    ).read_text()
    textured_locations = {int(value) for value in re.findall(r"@location\((\d+)\)", textured_block)}
    if not {0, 1, 2}.issubset(textured_locations) or any(value > 2 for value in textured_locations):
        raise AssertionError(
            f"textured vertex interface must expose exactly locations 0..2, got {sorted(textured_locations)}"
        )

    pending_source = (
        ROOT / "vetrace_render/src/wgpu_window/frame_pending_draws.rs"
    ).read_text()
    if "illegal read/write feedback loop" not in pending_source:
        raise AssertionError("render-target self-feedback guard is missing")

    print("active WGPU render-texture ABI validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
