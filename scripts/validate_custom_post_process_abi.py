#!/usr/bin/env python3
from pathlib import Path
import re

root = Path(__file__).resolve().parents[1]
rust = (root / 'vetrace_render/src/wgpu_window/gpu_uniform_types.rs').read_text()
pack_root = root / 'vetrace_render/src/wgpu_window/custom_post_process'
pack = '\n'.join(path.read_text() for path in sorted(pack_root.glob('*.rs')))
ssr = (root / 'vetrace_render/src/wgpu_window/screen_space_reflections.wgsl').read_text()
fxaa = (root / 'vetrace_render/src/wgpu_window/fxaa.wgsl').read_text()

required_rust = [
    'params: [[f32; 4]; 8]',
    'screen_time: [f32; 4]',
    'info: [f32; 4]',
    'view_proj: [[f32; 4]; 4]',
    'inverse_view_proj: [[f32; 4]; 4]',
    'camera_position: [f32; 4]',
    'camera_forward: [f32; 4]',
    'previous_view_proj: [[f32; 4]; 4]',
]
for field in required_rust:
    if field not in rust:
        raise SystemExit(f'missing Rust custom-post-process ABI field: {field}')

required_wgsl = [
    'view_proj: mat4x4<f32>',
    'inverse_view_proj: mat4x4<f32>',
    'camera_position: vec4<f32>',
    'camera_forward: vec4<f32>',
    'previous_view_proj: mat4x4<f32>',
]
for field in required_wgsl:
    if field not in ssr:
        raise SystemExit(f'missing screen-space-reflection WGSL ABI field: {field}')

for assignment in [
    'view_proj: camera.view_proj',
    'inverse_view_proj: camera.inverse_view_proj',
    'camera_position: camera.camera_position',
    'camera_forward: camera.camera_forward',
    'previous_view_proj: previous_view_proj.to_cols_array_2d()',
]:
    if assignment not in pack:
        raise SystemExit(f'custom-post-process camera data is not packed: {assignment}')


for required in [
    'binding: 4',
    '@group(0) @binding(4) var history_color: texture_2d<f32>;',
    'ensure_ssr_history_size',
    'copy_texture_to_texture',
    'temporal_neighborhood_bounds',
    'apply_temporal_ssr',
]:
    haystack = pack + ssr
    if required not in haystack:
        raise SystemExit(f'missing temporal SSR routing: {required}')

if 'custom_post_process_uniform_buffers[pass_index]' not in pack:
    raise SystemExit('custom post-process passes do not use stable per-pass uniform buffers')

# FXAA branches on sampled luminance. Any sampling after that branch must use an
# explicit LOD because implicit-derivative textureSample calls are forbidden in
# non-uniform control flow by WebGPU's WGSL validator. Using explicit LOD for all
# FXAA reads keeps the shader valid on both browser WebGPU and native WGPU.
if 'textureSample(' in fxaa:
    raise SystemExit('FXAA must use textureSampleLevel(..., 0.0) instead of textureSample')
if fxaa.count('textureSampleLevel(') < 9:
    raise SystemExit('FXAA explicit-LOD sampling coverage is incomplete')

print('custom post-process camera/SSR ABI validation passed')
