struct DdgiParams { probe_counts: vec4<u32>, volume_min: vec4<f32>, volume_spacing: vec4<f32>, atlas_size: vec4<u32>, rays_per_probe: u32, probe_inner_size: u32, distance_inner_size: u32, enabled: u32, normal_bias: f32, view_bias: f32, hysteresis: f32, max_ray_distance: f32, camera_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> ddgi: DdgiParams;
@group(0) @binding(1) var irradiance_in: texture_2d<f32>;
@group(0) @binding(2) var distance_in: texture_2d<f32>;
@group(0) @binding(3) var irradiance_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var distance_out: texture_storage_2d<rg16float, write>;
fn tile_origin(probe: u32, tile: u32, atlas_w: u32) -> vec2<u32> { let tiles_x = max(atlas_w / tile, 1u); return vec2<u32>((probe % tiles_x) * tile, (probe / tiles_x) * tile); }
fn wrap_coord(local: vec2<u32>, inner: u32) -> vec2<u32> { var p = local; if (p.x == 0u) { p.x = inner; } else if (p.x == inner + 1u) { p.x = 1u; } if (p.y == 0u) { p.y = inner; } else if (p.y == inner + 1u) { p.y = 1u; } return p; }
fn fix_irradiance(probe: u32, local: vec2<u32>) { let inner = max(ddgi.probe_inner_size, 1u); let tile = inner + 2u; let o = tile_origin(probe, tile, ddgi.atlas_size.x); let src = o + wrap_coord(local, inner); let dst = o + local; textureStore(irradiance_out, vec2<i32>(i32(dst.x), i32(dst.y)), textureLoad(irradiance_in, vec2<i32>(i32(src.x), i32(src.y)), 0)); }
fn fix_distance(probe: u32, local: vec2<u32>) { let inner = max(ddgi.distance_inner_size, 1u); let tile = inner + 2u; let o = tile_origin(probe, tile, ddgi.atlas_size.z); let src = o + wrap_coord(local, inner); let dst = o + local; let m = textureLoad(distance_in, vec2<i32>(i32(src.x), i32(src.y)), 0).rg; textureStore(distance_out, vec2<i32>(i32(dst.x), i32(dst.y)), vec4<f32>(m, 0.0, 0.0)); }
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) { if (ddgi.enabled == 0u || id.z >= ddgi.probe_counts.w) { return; } let pi = max(ddgi.probe_inner_size, 1u); let di = max(ddgi.distance_inner_size, 1u); if (id.x < pi + 2u && id.y < pi + 2u && (id.x == 0u || id.y == 0u || id.x == pi + 1u || id.y == pi + 1u)) { fix_irradiance(id.z, id.xy); } if (id.x < di + 2u && id.y < di + 2u && (id.x == 0u || id.y == 0u || id.x == di + 1u || id.y == di + 1u)) { fix_distance(id.z, id.xy); } }
