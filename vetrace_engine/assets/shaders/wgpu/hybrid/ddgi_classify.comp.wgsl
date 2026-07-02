struct DdgiParams { probe_counts: vec4<u32>, volume_min: vec4<f32>, volume_spacing: vec4<f32>, atlas_size: vec4<u32>, rays_per_probe: u32, probe_inner_size: u32, distance_inner_size: u32, enabled: u32, normal_bias: f32, view_bias: f32, hysteresis: f32, max_ray_distance: f32, camera_pos: vec4<f32> };
@group(0) @binding(0) var<uniform> ddgi: DdgiParams;
@group(0) @binding(1) var ray_distance: texture_2d<f32>;
@group(0) @binding(2) var<storage, read_write> probe_states: array<u32>;
fn grid(index: u32) -> vec3<u32> { let nx = max(ddgi.probe_counts.x, 1u); let ny = max(ddgi.probe_counts.y, 1u); return vec3<u32>(index % nx, (index / nx) % ny, index / max(nx * ny, 1u)); }
fn pos(index: u32) -> vec3<f32> { let g = grid(index); return ddgi.volume_min.xyz + vec3<f32>(f32(g.x), f32(g.y), f32(g.z)) * ddgi.volume_spacing.xyz; }
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) { let probe = id.x; if (ddgi.enabled == 0u || probe >= ddgi.probe_counts.w) { return; } var min_d = ddgi.max_ray_distance; var hit_count = 0u; for (var r = 0u; r < ddgi.rays_per_probe; r = r + 1u) { let d = textureLoad(ray_distance, vec2<i32>(i32(r), i32(probe)), 0).x; min_d = min(min_d, d); if (d < ddgi.max_ray_distance * 0.98) { hit_count = hit_count + 1u; } } let camera_relevant = distance(pos(probe), ddgi.camera_pos.xyz) < ddgi.max_ray_distance * 4.0; let geometry_relevant = hit_count > max(ddgi.rays_per_probe / 32u, 0u) && min_d > ddgi.normal_bias; probe_states[probe] = select(0u, 1u, camera_relevant || geometry_relevant); }
