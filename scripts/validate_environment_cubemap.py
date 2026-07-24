#!/usr/bin/env python3
"""Static cubemap/environment ABI and renderer-routing checks.

This intentionally uses only the Python standard library so it can run on
machines that do not have Cargo or naga installed. It catches the integration
mistakes that otherwise appear as WGPU validation errors at runtime.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    path = ROOT / relative
    return path.read_text(encoding="utf-8")


def read_tree(relative: str) -> str:
    root = ROOT / relative
    return "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(root.rglob("*.rs"))
    )


def fail(message: str) -> None:
    raise AssertionError(message)


def require(text: str, needle: str, context: str) -> None:
    if needle not in text:
        fail(f"{context}: missing {needle!r}")


def fields(text: str, struct_name: str) -> list[str]:
    match = re.search(rf"struct\s+{re.escape(struct_name)}\s*\{{(.*?)\n\}}", text, re.S)
    if not match:
        fail(f"missing struct {struct_name}")
    result: list[str] = []
    for line in match.group(1).splitlines():
        line = line.split("//", 1)[0].strip()
        field = re.match(r"(?:pub(?:\([^)]*\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:", line)
        if field:
            result.append(field.group(1))
    return result


def balanced(text: str, name: str) -> None:
    pairs = {"{": "}", "(": ")", "[": "]"}
    stack: list[tuple[str, int]] = []
    in_string = False
    escaped = False
    for index, char in enumerate(text):
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
            continue
        if char == '"':
            in_string = True
        elif char in pairs:
            stack.append((char, index))
        elif char in pairs.values():
            if not stack or pairs[stack[-1][0]] != char:
                fail(f"{name}: unbalanced {char!r} at byte {index}")
            stack.pop()
    if stack:
        fail(f"{name}: unclosed {stack[-1][0]!r} at byte {stack[-1][1]}")


def main() -> int:
    rust_gpu = read("vetrace_render/src/wgpu_window/environment/gpu_types.rs")
    wgsl_bindings = read("vetrace_render/src/wgpu_window/environment_bindings.wgsl")
    layout = read_tree("vetrace_render/src/wgpu_window/environment/resources")
    composition = read("vetrace_render/src/wgpu_window/default_fragment_shader.rs")
    pipeline = read("vetrace_render/src/wgpu_window/object_pipeline_builders.rs")
    scene_pass = read("vetrace_render/src/wgpu_window/frame_scene_pass.rs")
    custom_rust = read("vetrace_render/src/wgpu_backend/custom_shader_uniform.rs")
    material_bindings = read("vetrace_render/src/wgpu_window/default_material_bindings.wgsl")
    camera_rust = read("vetrace_render/src/wgpu_window/gpu_uniform_types.rs")
    sky = read("vetrace_render/src/wgpu_window/environment/shaders.rs")
    lighting = read("vetrace_render/src/wgpu_window/environment_lighting.wgsl")
    material_lighting = read("vetrace_render/src/wgpu_window/default_material_lighting.wgsl")
    fragment = read("vetrace_render/src/wgpu_window/default_material_fragment.wgsl")
    extraction = read_tree("vetrace_render/src/frame_extraction")

    expected_environment = ["slots_counts", "params0", "params1", "post_process"]
    if fields(rust_gpu, "EnvironmentUniform") != expected_environment:
        fail("Rust EnvironmentUniform field order changed")
    if fields(wgsl_bindings, "EnvironmentUniform") != expected_environment:
        fail("WGSL EnvironmentUniform field order differs from Rust")

    expected_probe = [
        "world_to_probe",
        "half_extents_blend",
        "capture_intensity",
        "slots_modes",
        "transition_params",
        "layer_masks",
    ]
    if fields(rust_gpu, "GpuReflectionProbe") != expected_probe:
        fail("Rust GpuReflectionProbe field order changed")
    if fields(wgsl_bindings, "ReflectionProbeGpu") != expected_probe:
        fail("WGSL ReflectionProbeGpu field order differs from Rust")

    for binding in range(6):
        require(layout, f"binding: {binding}", "environment bind-group layout")
        require(wgsl_bindings, f"@group(2) @binding({binding})", "environment WGSL")

    expected_composition = [
        'include_str!("environment_bindings.wgsl")',
        'include_str!("default_material_bindings.wgsl")',
        'include_str!("default_material_lighting.wgsl")',
        'include_str!("default_material_fragment.wgsl")',
        'include_str!("environment_lighting.wgsl")',
    ]
    positions = [composition.find(part) for part in expected_composition]
    if any(position < 0 for position in positions) or positions != sorted(positions):
        fail(f"default fragment composition order is unsafe: {positions}")

    require(
        pipeline,
        "bind_group_layouts: &[material_layout, camera_layout, environment_layout]",
        "object pipeline routing",
    )
    require(scene_pass, "pass.set_bind_group(2, &self.environment.environment_bind_group, &[]);", "scene pass")
    require(scene_pass, "pass.set_bind_group(1, &self.environment.environment_bind_group, &[]);", "sky pass")

    rust_tail = fields(custom_rust, "CustomShaderUniform")[-2:]
    wgsl_tail = fields(material_bindings, "VetraceCustomParams")[-2:]
    expected_tail = ["reflection_probe_indices", "reflection_probe_params"]
    if rust_tail != expected_tail or wgsl_tail != expected_tail:
        fail(f"custom uniform probe tail mismatch: Rust={rust_tail}, WGSL={wgsl_tail}")

    require(camera_rust, "inverse_view_proj: [[f32; 4]; 4]", "camera ABI")
    require(sky, "inverse_view_proj: mat4x4<f32>", "sky camera ABI")
    require(sky, "camera.inverse_view_proj", "sky direction reconstruction")

    require(extraction, ".is_some_and(|cubemap| cubemap.is_valid())", "asset extraction")
    require(rust_gpu, "const MAX_REFLECTION_PROBES: usize = 16;", "probe pool capacity")
    require(rust_gpu, "const ENVIRONMENT_CUBEMAP_FACE_SIZE: u32 = 256;", "cubemap pool resolution")
    require(rust_gpu, "const ENVIRONMENT_CUBEMAP_MIP_COUNT: u32 = 9;", "cubemap mip chain")
    require(rust_gpu, "const ENVIRONMENT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;", "cubemap pool format")
    require(layout, "depth_or_array_layers: ENVIRONMENT_CUBEMAP_CAPACITY * 6", "cube-array layers")
    require(layout, "fn environment_slot_pair(", "single-sided environment fallback")
    # The shadow path branches on per-fragment data (normal, cascade, and
    # adaptive PCF radius). Implicit-derivative comparison sampling is rejected
    # by browser WebGPU in non-uniform control flow, so all shadow comparisons
    # must use the explicit-level builtin.
    if "textureSampleCompare(" in material_lighting:
        fail("default material shadows must use textureSampleCompareLevel in browser-safe control flow")
    require(material_lighting, "textureSampleCompareLevel(", "explicit-level directional shadow sampling")

    require(lighting, "smooth_environment_transition", "cubemap crossfade")
    require(lighting, "fn reflection_probe_direction(", "box-projected parallax")
    require(lighting, "radiance = local_radiance / local_weight", "overlap normalization")
    require(lighting, "let coverage =", "probe-edge fallback coverage")
    require(fragment, "legacy_ambient_specular * (1.0 - cubemap_specular.a) + cubemap_specular.rgb", "material fallback blend")

    chunks = [wgsl_bindings, material_bindings, material_lighting, fragment, lighting]
    combined = "\n".join(chunks)
    functions = re.findall(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", combined)
    duplicates = sorted({name for name in functions if functions.count(name) > 1})
    if duplicates:
        fail(f"duplicate functions in concatenated default fragment WGSL: {duplicates}")

    balanced(combined, "concatenated default fragment WGSL")
    capture = read_tree("vetrace_render/src/wgpu_window/environment/capture")
    prefilter = read("vetrace_render/src/wgpu_window/environment_prefilter.wgsl")
    component = read("vetrace_render/src/component_environment.rs")
    frame_render = read("vetrace_render/src/wgpu_window/frame_render.rs")

    require(component, "pub enum ReflectionProbeCaptureMode", "public capture API")
    for mode in ["Imported", "Baked", "OnDemand", "Realtime"]:
        require(component, mode, "capture mode variants")
    require(component, "pub fn request_capture(&mut self)", "on-demand capture request")
    require(component, "pub capture_include_layers: u32", "capture-only include layers")
    require(component, "pub capture_exclude_layers: u32", "capture-only exclude layers")
    require(component, "pub capture_shadows: bool", "capture-camera shadow policy")
    require(capture, "probe.capture_include_layers & !probe.capture_exclude_layers", "capture-only layer routing")
    require(capture, "ReflectionCapturePhase::Capturing", "six-face capture state")
    require(capture, "let end_face = (next_face + face_budget).min(6);", "budgeted six-face scheduling")
    require(capture, "ReflectionCapturePhase::Filtering", "incremental prefilter state")
    require(capture, "probe.entity.0", "self-exclusion routing")
    require(capture, "capture_environment_bind_group", "recursive reflection prevention")
    require(capture, "initial_transition_started.is_none()", "capture transition serialization")
    require(frame_render, "render_reflection_probe_capture_work", "frame capture routing")
    require(rust_gpu, "ENVIRONMENT_RUNTIME_SLOT_BASE", "double-buffered runtime slots")
    require(rust_gpu, "transition_to: Option<usize>", "captured cubemap crossfade")
    require(prefilter, "importance_sample_ggx", "GGX cubemap prefilter")
    require(prefilter, "textureSampleLevel(source_cubemap", "cubemap prefilter source")
    require(wgsl_bindings, "@group(2) @binding(4)", "BRDF LUT texture binding")
    require(wgsl_bindings, "@group(2) @binding(5)", "BRDF LUT sampler binding")
    require(lighting, "environment_brdf_lut", "split-sum BRDF LUT usage")
    require(layout, "create_environment_brdf_lut", "BRDF LUT generation")
    require(fragment, "environment.params1.w >= 0.5", "linear HDR capture bypass")
    require(fragment, "environment.params1.w < 0.5", "capture fog exclusion")
    require(fragment, "if (i == 0) {", "capture/main shadow routing")
    require(capture, "render_reflection_capture_shadow_passes", "capture-camera shadow passes")
    require(rust_gpu, "shadow_camera_buffers: Vec<wgpu::Buffer>", "per-face capture shadow camera buffers")
    require(frame_render, "Reflection captures may render their own camera-relative shadow maps", "main shadow restore ordering")

    for relative in [
        "vetrace_render/src/wgpu_window/environment/shaders.rs",
        "vetrace_render/src/wgpu_window/environment/probe_selection.rs",
        "vetrace_render/src/wgpu_window/environment_prefilter.wgsl",
    ]:
        balanced(read(relative), relative)
    for relative in [
        "vetrace_render/src/wgpu_window/environment/resources",
        "vetrace_render/src/wgpu_window/environment/capture",
    ]:
        balanced(read_tree(relative), relative)

    print("environment cubemap validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as error:
        print(f"environment cubemap validation failed: {error}", file=sys.stderr)
        raise SystemExit(1)
