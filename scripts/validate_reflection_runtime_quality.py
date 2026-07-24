#!/usr/bin/env python3
"""Static regression checks for production reflection-system hardening."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")

def require(path: str, needle: str) -> None:
    text = read(path)
    if needle not in text:
        raise SystemExit(f"{path}: missing {needle!r}")

# Real CI and semantic shader validation.
require(".github/workflows/reflection-quality.yml", "cargo check --locked --workspace --all-targets --all-features")
require(".github/workflows/reflection-quality.yml", "cargo test --locked --workspace --all-features")
require("vetrace_render/tests/wgsl_validation.rs", "naga::front::wgsl::parse_str")
require("vetrace_render/tests/wgsl_validation.rs", "Validator::new")

# Runtime budgets, spatial lookup, and safe large-volume fallback.
for needle in (
    "reflection_capture_probe_budget_per_frame",
    "reflection_prefilter_mips_per_frame",
    "reflection_max_resident_runtime_probes",
    "reflection_capture_distance_limit",
    "reflection_probe_grid_cell_size",
):
    require("vetrace_render/src/resources/settings.rs", needle)
require("vetrace_render/src/wgpu_window/environment/spatial_index.rs", "MAX_CELLS_PER_PROBE")
require("vetrace_render/src/wgpu_window/environment/spatial_index.rs", "MAX_QUERY_CELLS")
require("vetrace_render/src/wgpu_window/environment/gpu_types.rs", "oversized: Vec<u32>")
require("vetrace_render/src/wgpu_window/environment/capture/state.rs", "reflection_probe_evictions_total")

# Automatic invalidation, asset revisions, and texture-cache refresh.
require("vetrace_render/src/component_environment.rs", "ReflectionProbeInvalidationMode")
require("vetrace_render/src/component_environment.rs", "invalidation_delay_seconds")
require("vetrace_render/src/wgpu_window/environment/gpu_types.rs", "observe_scene_signature")
require("vetrace_render/src/frame_extraction/build_frame.rs", "reflection_layer_signatures")
require("vetrace_render/src/frame_extraction/reflection_signatures.rs", "texture.revision")
require("vetrace_render/src/resources/assets.rs", "pub revision: u64")
require("vetrace_render/src/resources/assets.rs", "pub fn touch_texture")
require("vetrace_render/src/resources/assets.rs", "pub fn touch_mesh")
require("vetrace_render/src/wgpu_window/draw_cache.rs", "texture_cache_revisions")

# Capture coverage and camera-correct shadows.
require("vetrace_render/src/component_environment.rs", "capture_transparent")
require("vetrace_render/src/component_environment.rs", "capture_custom_materials")
require("vetrace_render/src/component_environment.rs", "capture_shadows")
require("vetrace_render/src/component_custom_shader.rs", "CustomShaderReflectionCaptureMode")
require("vetrace_render/src/wgpu_window/environment/capture/shadow_passes.rs", "render_reflection_capture_shadow_passes")
require("vetrace_render/src/wgpu_window/environment/gpu_types.rs", "shadow_camera_bind_groups")

# HDR/offline environment import and generic editor capture command.
require("vetrace_render/src/resources/cubemap.rs", "prefiltered_rgba16f_mips")
require("vetrace_render/src/resources/cubemap_import.rs", "load_equirectangular")
require("vetrace_render/src/resources/cubemap_import.rs", "load_ktx2")
require("vetrace_render/src/resources/cubemap_import.rs", "0x32, 0x30, 0xBB")
require("Cargo.lock", '"image",')
require("Cargo.lock", '"naga",')
require("vetrace_render/src/lib.rs", "ReflectionProbeCaptureRequests")
require("vetrace_render/src/resources/reflection_control.rs", "ReflectionProbeCaptureRequests")
require("vetrace_editor/src/active_editor/mod.rs", "request_selected_reflection_probe_capture")

# Temporal SSR lifecycle and memory accounting.
require("vetrace_render/src/resources/post_process.rs", "temporal_weight")
require("vetrace_render/src/wgpu_window/screen_space_reflections.wgsl", "apply_temporal_ssr")
require("vetrace_render/src/wgpu_window/lifecycle.rs", "self.post_process.ssr_history_valid = false")
require("vetrace_render/src/wgpu_window/gpu_memory_reporting.rs", "self.post_process.ssr_history")

print("reflection runtime quality validation passed")
