# Renderer feature completion tasks

This document turns the current renderer-policy vocabulary into implementation tasks for the features that are either partially implemented or policy-only today. Each task includes the Rust integration points and shader work required to make the feature real instead of merely selectable.

## Current gap summary

| Area | Current state | Completion target |
| --- | --- | --- |
| AO / SSAO / GTAO / RTAO | Policy enums exist, but no dedicated AO render target, shader, pipeline, dispatch, or compositor input exists. | Add AO textures, pipelines, shaders, policy-gated dispatch, denoise/composite integration, and profiler status. |
| SSR | SSR logic exists in shader code, but it is not exposed as a dedicated renderer pass with a named feature target and history ownership. | Promote SSR to an explicit screen-space reflection pass used before RT fallback. |
| GI | SDFGI and one-bounce RTGI are wired, but GI still mixes real paths with placeholders/simple approximations. | Finish a common GI buffer contract for baked/probe/SDFGI/RTGI/path-traced GI and expose consistent policy dispatch. |
| Feature status | Policy methods are reported, but not every method corresponds to a dispatched pass yet. | Only report methods as active when their concrete pass ran, and separately report requested policy methods. |

## Task 1: Add AO renderer resources and policy-gated dispatch

### Rust ownership

Primary files:

- `vetrace_engine/src/rendering/wgpu_renderer/renderer.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/types.rs`

### Required Rust changes

Add AO targets to `WgpuRenderer`:

```rust
// renderer.rs
ambient_occlusion_texture: Texture,
ambient_occlusion_view: TextureView,
ambient_occlusion_history_texture: Texture,
ambient_occlusion_history_view: TextureView,
ambient_occlusion_pipeline: Option<ComputePipeline>,
ambient_occlusion_bind_group_layout: BindGroupLayout,
ambient_occlusion_bind_group: BindGroup,
ambient_occlusion_params_buffer: Buffer,
```

Add an AO params ABI:

```rust
// types.rs
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct AmbientOcclusionParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub radius: f32,
    pub intensity: f32,
    pub temporal_blend: f32,
    pub method: u32,
    pub frame_number: u32,
    pub sample_count: u32,
    pub _pad: [u32; 2],
}

pub const AO_METHOD_OFF: u32 = 0;
pub const AO_METHOD_SSAO: u32 = 1;
pub const AO_METHOD_GTAO: u32 = 2;
pub const AO_METHOD_RTAO: u32 = 3;
```

Map policy to shader ABI once per frame:

```rust
fn ao_method_to_abi(method: AmbientOcclusionMethod) -> u32 {
    match method {
        AmbientOcclusionMethod::Off => AO_METHOD_OFF,
        AmbientOcclusionMethod::SSAO => AO_METHOD_SSAO,
        AmbientOcclusionMethod::GTAO => AO_METHOD_GTAO,
        AmbientOcclusionMethod::RTAO => AO_METHOD_RTAO,
    }
}
```

Dispatch only when the policy-selected AO method has a concrete shader path:

```rust
let ao_method = ao_method_to_abi(policy.ambient_occlusion);
let ao_active = !self.is_2d && ao_method != AO_METHOD_OFF;
if ao_active {
    let ao_params = AmbientOcclusionParams {
        inv_view_proj: params.inv_view_proj,
        proj: current_vp.to_cols_array_2d(),
        camera_pos: [params.camera_pos[0], params.camera_pos[1], params.camera_pos[2], 0.0],
        radius: match policy.ambient_occlusion {
            AmbientOcclusionMethod::SSAO => 0.75,
            AmbientOcclusionMethod::GTAO => 1.5,
            AmbientOcclusionMethod::RTAO => params.gi_max_distance.min(4.0),
            AmbientOcclusionMethod::Off => 0.0,
        },
        intensity: 1.0,
        temporal_blend: self.post_fx_uniforms.gi_temporal_blend.clamp(0.0, 0.98),
        method: ao_method,
        frame_number: self.frame_number.max(0) as u32,
        sample_count: if self.adaptive_quality < 0.67 { 6 } else if self.adaptive_quality < 0.9 { 10 } else { 16 },
        _pad: [0; 2],
    };
    self.queue.write_buffer(&self.ambient_occlusion_params_buffer, 0, bytemuck::bytes_of(&ao_params));

    let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("ambient_occlusion"),
        timestamp_writes: None,
    });
    if let Some(pipeline) = &self.ambient_occlusion_pipeline {
        cpass.set_pipeline(pipeline);
        cpass.set_bind_group(0, &self.ambient_occlusion_bind_group, &[]);
        cpass.dispatch_workgroups((self.width + 7) / 8, (self.height + 7) / 8, 1);
    }
}
feature_status.ambient_occlusion_method = if ao_active {
    policy.ambient_occlusion
} else {
    AmbientOcclusionMethod::Off
};
```

### Required WGSL shader

Create `vetrace_engine/assets/shaders/wgpu/hybrid/ambient_occlusion.comp.wgsl`:

```wgsl
struct AmbientOcclusionParams {
    inv_view_proj: mat4x4<f32>,
    proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    radius: f32,
    intensity: f32,
    temporal_blend: f32,
    method: u32,
    frame_number: u32,
    sample_count: u32,
    _pad: vec2<u32>,
};

const AO_METHOD_OFF: u32 = 0u;
const AO_METHOD_SSAO: u32 = 1u;
const AO_METHOD_GTAO: u32 = 2u;
const AO_METHOD_RTAO: u32 = 3u;

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var ao_history_tex: texture_2d<f32>;
@group(0) @binding(3) var ao_out: texture_storage_2d<r16float, write>;
@group(0) @binding(4) var<uniform> ao_params: AmbientOcclusionParams;

fn hash12(p: vec2<u32>) -> f32 {
    var x = p.x * 1664525u + p.y * 1013904223u + ao_params.frame_number * 747796405u;
    x = ((x >> 16u) ^ x) * 2246822519u;
    x = ((x >> 13u) ^ x) * 3266489917u;
    x = (x >> 16u) ^ x;
    return f32(x & 0x00ffffffu) / f32(0x01000000u);
}

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    var world = ao_params.inv_view_proj * clip;
    world = world / world.w;
    return world.xyz;
}

fn ssao(pixel: vec2<i32>, dims: vec2<u32>, world: vec3<f32>, normal: vec3<f32>) -> f32 {
    var occ = 0.0;
    let samples = max(ao_params.sample_count, 1u);
    let base_angle = hash12(vec2<u32>(pixel)) * 6.2831853;
    for (var i = 0u; i < samples; i = i + 1u) {
        let fi = f32(i) + 0.5;
        let r = sqrt(fi / f32(samples)) * ao_params.radius;
        let a = base_angle + fi * 2.3999632;
        let offset = vec2<f32>(cos(a), sin(a)) * r * 24.0;
        let sp = pixel + vec2<i32>(offset);
        if (any(sp < vec2<i32>(0)) || any(sp >= vec2<i32>(dims))) { continue; }
        let sd = textureLoad(depth_tex, sp, 0).x;
        if (sd >= 0.9999) { continue; }
        let sw = reconstruct_world(sp, dims, sd);
        let v = sw - world;
        let dist = length(v);
        let facing = max(dot(normalize(v), normal), 0.0);
        let range = smoothstep(ao_params.radius, 0.0, dist);
        occ = occ + (1.0 - facing) * range;
    }
    return clamp(1.0 - (occ / f32(samples)) * ao_params.intensity, 0.0, 1.0);
}

fn gtao(pixel: vec2<i32>, dims: vec2<u32>, world: vec3<f32>, normal: vec3<f32>) -> f32 {
    var horizon_occ = 0.0;
    let directions = max(ao_params.sample_count / 2u, 4u);
    let base_angle = hash12(vec2<u32>(pixel)) * 6.2831853;
    for (var d = 0u; d < directions; d = d + 1u) {
        let a = base_angle + (f32(d) / f32(directions)) * 6.2831853;
        let dir = vec2<f32>(cos(a), sin(a));
        var max_horizon = 0.0;
        for (var s = 1u; s <= 4u; s = s + 1u) {
            let sp = pixel + vec2<i32>(dir * f32(s) * 8.0);
            if (any(sp < vec2<i32>(0)) || any(sp >= vec2<i32>(dims))) { continue; }
            let sd = textureLoad(depth_tex, sp, 0).x;
            if (sd >= 0.9999) { continue; }
            let sw = reconstruct_world(sp, dims, sd);
            let v = sw - world;
            let dist = max(length(v), 1e-4);
            let horizon = max(dot(normalize(v), normal), 0.0) * smoothstep(ao_params.radius, 0.0, dist);
            max_horizon = max(max_horizon, horizon);
        }
        horizon_occ = horizon_occ + max_horizon;
    }
    return clamp(1.0 - (horizon_occ / f32(directions)) * ao_params.intensity, 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (ao_params.method == AO_METHOD_OFF) {
        textureStore(ao_out, pixel, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) {
        textureStore(ao_out, pixel, vec4<f32>(1.0, 0.0, 0.0, 0.0));
        return;
    }
    let world = reconstruct_world(pixel, dims, depth);
    let normal = normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
    var ao = select(ssao(pixel, dims, world, normal), gtao(pixel, dims, world, normal), ao_params.method == AO_METHOD_GTAO);
    if (ao_params.method == AO_METHOD_RTAO) {
        // Until a true BVH-backed RTAO kernel is added, fall back to GTAO but keep the ABI distinct.
        ao = gtao(pixel, dims, world, normal);
    }
    let history = textureLoad(ao_history_tex, pixel, 0).x;
    ao = mix(ao, history, clamp(ao_params.temporal_blend, 0.0, 0.98));
    textureStore(ao_out, pixel, vec4<f32>(ao, 0.0, 0.0, 0.0));
}
```

### Acceptance criteria

- `RendererPolicy` selecting SSAO/GTAO/RTAO causes an AO dispatch.
- The compositor multiplies indirect/ambient lighting by the AO texture.
- `RendererHybridFeatureStatus` distinguishes requested AO method from active AO method if the pipeline is unavailable.
- A debug view can display AO as grayscale.

## Task 2: Promote SSR to a dedicated feature pass

### Rust ownership

Primary files:

- `renderer.rs`
- `renderer_impl.inc.rs`
- `types.rs`

### Required Rust changes

Add SSR targets and history:

```rust
ssr_texture: Texture,
ssr_view: TextureView,
ssr_history_texture: Texture,
ssr_history_view: TextureView,
ssr_pipeline: Option<ComputePipeline>,
ssr_bind_group_layout: BindGroupLayout,
ssr_bind_group: BindGroup,
ssr_params_buffer: Buffer,
```

Dispatch before RT reflections:

```rust
let ssr_active = matches!(
    policy.reflections,
    ReflectionMethod::SSR | ReflectionMethod::SsrThenRtFallback
);
if ssr_active {
    // write params, dispatch ssr_pipeline, copy ssr_texture -> ssr_history_texture after composite
}
feature_status.reflection_method = if ssr_active {
    policy.reflections
} else if feature_status.hybrid_rt_reflections_active {
    ReflectionMethod::Raytraced
} else {
    ReflectionMethod::Probe
};
```

### Required WGSL shader

Create `vetrace_engine/assets/shaders/wgpu/hybrid/ssr.comp.wgsl` by extracting the existing SSR walk from `pathtrace.comp.wgsl` / `rt_reflections.comp.wgsl` into a standalone compute pass that writes `rgba16float` color plus confidence in alpha:

```wgsl
@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var albedo_tex: texture_2d<f32>;
@group(0) @binding(3) var color_history_tex: texture_2d<f32>;
@group(0) @binding(4) var ssr_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var<uniform> ssr_params: SsrParams;

struct SsrParams {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
    max_distance: f32,
    thickness: f32,
    stride_px: f32,
    max_steps: u32,
};

// Use the same reconstruct/project/depth-thickness checks as rt_reflections.comp.wgsl.
```

### Acceptance criteria

- Raster and hybrid modes can use SSR without enabling RT reflection dispatch.
- Hybrid high can run SSR first and RT only for low-confidence/important pixels.
- Reflection history ownership is documented and only one pass writes each history texture.

## Task 3: Finish RTAO as a BVH-backed AO path

### Required Rust changes

Reuse the hybrid RT bind group shape or create an AO-specific bind group including:

- depth texture,
- normal texture,
- object buffer,
- triangle buffer,
- BVH buffers,
- material buffer,
- AO params,
- AO output texture.

Dispatch only when:

```rust
matches!(policy.ambient_occlusion, AmbientOcclusionMethod::RTAO)
    && hardware.rt_shadows
    && self.hybrid_rt_shadow_pipeline.is_some()
```

### Required WGSL shader

Create `vetrace_engine/assets/shaders/wgpu/experimental/hybrid_effects/rt_ao.comp.wgsl`:

```wgsl
// Trace short hemisphere rays against the existing object/TLAS and triangle/BLAS buffers.
// Output single-channel visibility in R16Float.
// Start with 1-4 rays/pixel, blue-noise rotated, temporally accumulated by the AO pass.
```

### Acceptance criteria

- RTAO no longer aliases to GTAO.
- RTAO respects `max_traversal_steps`, `min_ray_offset`, and adaptive quality.
- Low/Indoor profiles never dispatch RTAO.

## Task 4: Complete the GI buffer contract

### Required Rust changes

Make `gi_buffer_texture` the single compositor input for all non-path-traced GI methods:

```rust
match policy.gi {
    GiMethod::Off => clear_gi_buffer_to_black(),
    GiMethod::BakedLightmap | GiMethod::LightProbes => dispatch_probe_gi_or_upload_baked(),
    GiMethod::SDFGI => dispatch_sdfgi_then_resolve_to_gi_buffer(),
    GiMethod::RTGIOneBounce => dispatch_hybrid_rtgi_then_resolve_to_gi_buffer(),
    GiMethod::PathTraced => skip_gi_buffer_for_pathtrace_primary(),
}
```

### Required WGSL shader

Create `vetrace_engine/assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl`:

```wgsl
@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var sdfgi_tex: texture_3d<f32>;
@group(0) @binding(3) var rtgi_tex: texture_2d<f32>;
@group(0) @binding(4) var lightmap_tex: texture_2d<f32>;
@group(0) @binding(5) var gi_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(6) var<uniform> gi_params: GiResolveParams;

// Resolve the selected GI method to a common RGB irradiance buffer.
```

### Acceptance criteria

- `hybrid_compose.comp.wgsl` only samples `gi_buffer` and does not care which GI method produced it.
- Baked/probe/SDFGI/RTGI debug views all show the same output convention.
- RTGI temporal accumulation is owned by GI resolve, not the compositor.

## Task 5: Make feature status truthful

### Required Rust changes

Split requested policy from active execution:

```rust
pub struct RendererHybridFeatureStatus {
    pub requested_primary_visibility_method: PrimaryVisibilityMethod,
    pub active_primary_visibility_method: PrimaryVisibilityMethod,
    pub requested_shadow_method: ShadowMethod,
    pub active_shadow_method: ShadowMethod,
    pub requested_reflection_method: ReflectionMethod,
    pub active_reflection_method: ReflectionMethod,
    pub requested_ambient_occlusion_method: AmbientOcclusionMethod,
    pub active_ambient_occlusion_method: AmbientOcclusionMethod,
    pub requested_gi_method: GiMethod,
    pub active_gi_method: GiMethod,
    pub requested_transparency_method: TransparencyMethod,
    pub active_transparency_method: TransparencyMethod,
    // legacy booleans stay until HUD/users migrate
}
```

### Acceptance criteria

- If policy requests RTAO but the RTAO pipeline is missing, requested is `RTAO` and active is `GTAO` or `Off`.
- If cinematic pathtrace pipeline fails to compile, requested primary is `PathTraced` and active primary is the fallback actually used.
- HUD displays requested and active methods side-by-side.

## Recommended implementation order

1. **AO target + SSAO/GTAO shader** because AO is currently policy-only.
2. **SSR standalone pass** because SSR code already exists and mostly needs ownership/pipeline cleanup.
3. **GI resolve buffer contract** because it will simplify compositor and debug behavior.
4. **RTAO** because it needs BVH traversal reuse and denoising.
5. **Truthful requested/active status split** after all concrete fallbacks exist.
