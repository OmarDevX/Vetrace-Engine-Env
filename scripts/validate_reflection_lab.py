#!/usr/bin/env python3
"""Regression checks for generic reflection-lab integration."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
window = (ROOT / "vetrace_render/src/wgpu_window.rs").read_text(encoding="utf-8")
example = (ROOT / "vetrace_render/examples/cubemap_environment.rs").read_text(encoding="utf-8")
capture_root = ROOT / "vetrace_render/src/wgpu_window/environment/capture"
capture = "\n".join(
    path.read_text(encoding="utf-8") for path in sorted(capture_root.glob("*.rs"))
)
gpu_types = (ROOT / "vetrace_render/src/wgpu_window/environment/gpu_types.rs").read_text(encoding="utf-8")
ssr = (ROOT / "vetrace_render/src/wgpu_window/screen_space_reflections.wgsl").read_text(encoding="utf-8")
bloom = (ROOT / "vetrace_render/src/wgpu_window/bloom.wgsl").read_text(encoding="utf-8")

backend_import = window.split("use crate::backend::{", 1)[1].split("};", 1)[0]
if "RenderReflectionProbe" not in backend_import:
    raise SystemExit("wgpu_window.rs must import RenderReflectionProbe for capture.rs")

if "ScreenSpaceReflections" not in example:
    raise SystemExit("reflection lab must use the reusable ScreenSpaceReflections resource")
if "CustomPostProcessStack" in example or "reflection_lab_bloom.wgsl" in example:
    raise SystemExit("reflection lab still owns renderer-generic SSR/bloom implementation")

for required in (
    "camera_buffers: Vec<wgpu::Buffer>",
    "camera_bind_groups: Vec<wgpu::BindGroup>",
):
    if required not in gpu_types:
        raise SystemExit(f"missing safe multi-face capture resource: {required}")
if "reflection_capture_faces_per_frame" not in capture:
    raise SystemExit("capture face budget is not connected to the renderer")
if "configured_sample_count.clamp(16, 256)" not in capture:
    raise SystemExit("reflection prefilter quality is not configurable")

for required in ("depth_continuity", "distance_confidence", "facing", "post.p2.x"):
    if required not in ssr:
        raise SystemExit(f"generic SSR confidence hardening missing: {required}")
for forbidden in ("directions[i]", "array<vec2<f32>, 8>"):
    if forbidden in bloom:
        raise SystemExit(f"generic bloom contains unsupported dynamic local-array indexing: {forbidden}")
if "built_in_bloom_pass" not in (ROOT / "vetrace_render/src/frame_extraction/post_processing.rs").read_text(encoding="utf-8"):
    raise SystemExit("PostProcessing::bloom is not routed into a generic renderer pass")

print("reflection lab generic-integration validation passed")
