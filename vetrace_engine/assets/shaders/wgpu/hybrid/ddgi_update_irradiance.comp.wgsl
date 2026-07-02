const DDGI_PI: f32 = 3.14159265359;
struct DdgiParams { probe_counts: vec4<u32>, volume_min: vec4<f32>, volume_spacing: vec4<f32>, atlas_size: vec4<u32>, rays_per_probe: u32, probe_inner_size: u32, distance_inner_size: u32, enabled: u32, normal_bias: f32, view_bias: f32, hysteresis: f32, max_ray_distance: f32, camera_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> ddgi: DdgiParams;
@group(0) @binding(1) var ray_radiance: texture_2d<f32>;
@group(0) @binding(2) var previous_irradiance: texture_2d<f32>;
@group(0) @binding(3) var irradiance_out: texture_storage_2d<rgba16float, write>;
fn oct_decode(e: vec2<f32>) -> vec3<f32> { var v = vec3<f32>(e.x, e.y, 1.0 - abs(e.x) - abs(e.y)); if (v.z < 0.0) { let xy = (1.0 - abs(v.yx)) * sign(v.xy); v = vec3<f32>(xy, v.z); } return normalize(v); }
fn probe_tile_origin(probe: u32, tile: u32, atlas_w: u32) -> vec2<u32> { let tiles_x = max(atlas_w / tile, 1u); return vec2<u32>((probe % tiles_x) * tile, (probe / tiles_x) * tile); }
fn spherical_fibonacci(i: u32, n: u32) -> vec3<f32> { let count = max(f32(n), 1.0); let phi = 2.39996322972865332 * f32(i); let z = 1.0 - (2.0 * (f32(i) + 0.5) / count); let r = sqrt(max(0.0, 1.0 - z * z)); return vec3<f32>(cos(phi) * r, sin(phi) * r, z); }
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let inner = max(ddgi.probe_inner_size, 1u); let tile = inner + 2u; let probe = id.z; if (ddgi.enabled == 0u || probe >= ddgi.probe_counts.w || id.x >= inner || id.y >= inner) { return; }
    let atlas_xy = probe_tile_origin(probe, tile, ddgi.atlas_size.x) + vec2<u32>(id.xy) + vec2<u32>(1u);
    let uv = ((vec2<f32>(id.xy) + vec2<f32>(0.5)) / f32(inner)) * 2.0 - vec2<f32>(1.0);
    let dir = oct_decode(uv);
    var weighted = vec3<f32>(0.0); var weight_sum = 0.0;
    for (var r = 0u; r < ddgi.rays_per_probe; r = r + 1u) { let ray_dir = spherical_fibonacci(r, ddgi.rays_per_probe); let w = max(dot(dir, ray_dir), 0.0); let sample = textureLoad(ray_radiance, vec2<i32>(r, probe), 0).rgb; weighted = weighted + sample * w; weight_sum = weight_sum + w; }
    let integrated = weighted / max(weight_sum, 1.0e-4) * DDGI_PI;
    let prev = textureLoad(previous_irradiance, vec2<i32>(atlas_xy), 0).rgb;
    let h = clamp(ddgi.hysteresis, 0.0, 0.98);
    textureStore(irradiance_out, vec2<i32>(atlas_xy), vec4<f32>(mix(integrated, prev, h), 1.0));
}
