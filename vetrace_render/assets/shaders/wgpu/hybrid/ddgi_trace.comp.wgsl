// DDGI probe ray tracing pass. Intended to be concatenated after
// pbr_lighting.wgsl and bvh_traversal.wgsl by the renderer.
const T_EPS: f32 = 0.002;
const INF_T: f32 = 1.0e20;
const PI: f32 = 3.14159265359;

struct Params {
    camera_pos: vec4<f32>, camera_front: vec4<f32>, camera_up: vec4<f32>, camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>, fov: f32, num_objects: i32, is_fisheye: i32, _pad0: i32,
    skycolor: vec4<f32>, taa_jitter: vec2<f32>, current_time: f32, frame_number: i32,
    selected_index: i32, max_bounces: i32, light_samples: i32, dir_shadow_samples: i32,
    shadow_mode: u32, raytraced_shadows_enabled: u32, shadow_quality: u32, max_shadow_rays: u32,
    emissive_shadow_samples: u32, directional_shadow_samples: u32, cloud_object_shadows_enabled: u32,
    max_rt_shadow_distance: f32, rt_shadow_ray_t_max: f32, min_soft_shadow_radius: f32,
    raytraced_reflections_enabled: u32, _pad_reflections: u32,
    inv_view_proj: mat4x4<f32>, prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>, dir_light_color: vec4<f32>, sky_occlusion: f32,
    total_triangles: u32, total_bvh_nodes: u32, total_tri_bvh_nodes: u32,
    dof_aperture: f32, dof_focus_dist: f32, dof_enable: u32, _pad_dof: u32,
    atmosphere: u32, atmo_count: u32, cloud_count: u32, atmosphere_mode: u32,
    atmosphere_sun_controls: vec4<f32>,
    cloud_history_weight: f32, cloud_sample_count: u32, cloud_temporal_quality: u32, cloud_shadow_mode: u32,
    renderer_mode: u32, rt_debug_view: u32, rt_debug_counters: u32, max_traversal_steps: u32,
    max_transparent_surfaces: u32, shadow_max_distance: f32, reflection_max_distance: f32, gi_max_distance: f32,
    min_ray_offset: f32,
};

struct DdgiParams {
    probe_counts: vec4<u32>,        // xyz counts, w total probes
    volume_min: vec4<f32>,         // xyz origin
    volume_spacing: vec4<f32>,     // xyz spacing
    atlas_size: vec4<u32>,         // xy irradiance, zw distance atlas sizes
    rays_per_probe: u32,
    probe_inner_size: u32,
    distance_inner_size: u32,
    enabled: u32,
    normal_bias: f32,
    view_bias: f32,
    hysteresis: f32,
    max_ray_distance: f32,
    camera_pos: vec4<f32>,
};

@group(0) @binding(8) var<uniform> params: Params;
@group(0) @binding(21) var rt_material_texture: texture_2d<f32>;
@group(0) @binding(22) var material_sampler: sampler;
@group(1) @binding(0) var<uniform> ddgi: DdgiParams;
@group(1) @binding(1) var<storage, read> probe_offsets: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read> probe_states: array<u32>;
@group(1) @binding(3) var ray_radiance_out: texture_storage_2d<rgba16float, write>;
@group(1) @binding(4) var ray_distance_out: texture_storage_2d<rg16float, write>;
@group(1) @binding(5) var previous_irradiance: texture_2d<f32>;

// Mirrors ddgi_atlas_coord()/oct_encode() in gi_resolve.comp.wgsl. Kept as a
// separate copy because this shader is concatenated into a different bind
// group / file than the resolve pass.
fn ddgi_oct_wrap(v: vec2<f32>) -> vec2<f32> {
    return (vec2<f32>(1.0) - abs(v.yx)) * select(vec2<f32>(-1.0), vec2<f32>(1.0), v >= vec2<f32>(0.0));
}
fn ddgi_oct_encode(n_in: vec3<f32>) -> vec2<f32> {
    var n = n_in / max(abs(n_in.x) + abs(n_in.y) + abs(n_in.z), 1.0e-6);
    if (n.z < 0.0) {
        n = vec3<f32>(ddgi_oct_wrap(n.xy), n.z);
    }
    return n.xy * 0.5 + vec2<f32>(0.5);
}
fn ddgi_trace_atlas_coord(probe_index: u32, dir: vec3<f32>, tile_texels: vec2<u32>, atlas_dims: vec2<u32>) -> vec2<i32> {
    let safe_tile = max(tile_texels, vec2<u32>(3u));
    let inner = max(safe_tile - vec2<u32>(2u), vec2<u32>(1u));
    let probes_per_row = max(atlas_dims.x / max(safe_tile.x, 1u), 1u);
    let tile = vec2<u32>(probe_index % probes_per_row, probe_index / probes_per_row);
    let oct = ddgi_oct_encode(normalize(dir));
    let texel = vec2<u32>(clamp(oct * vec2<f32>(inner - vec2<u32>(1u)), vec2<f32>(0.0), vec2<f32>(inner - vec2<u32>(1u))));
    let coord = min(tile * safe_tile + texel + vec2<u32>(1u), atlas_dims - vec2<u32>(1u));
    return vec2<i32>(i32(coord.x), i32(coord.y));
}

// The "infinite bounce" trick: sample the volume's own last-converged
// irradiance at the ray hit point and feed it back in as incoming light on
// the next trace. Without this, probe rays only ever see direct light, so
// DDGI can brighten a scene but never carries bounce color between surfaces.
// Nearest-probe (not trilinear) is used deliberately: this runs once per
// traced ray already, and the atlas is further smoothed spatially by the
// bilinear resolve pass that consumes it.
fn sample_indirect_bounce(hit_pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    // During a full DDGI refresh the previous irradiance atlas may contain
    // undefined/cleared data and the CPU sets hysteresis to 0.0. Do not feed
    // that atlas back into probe tracing until at least one clean update has
    // populated it.
    if (ddgi.hysteresis <= 0.0) {
        return vec3<f32>(0.0);
    }
    let counts = max(ddgi.probe_counts.xyz, vec3<u32>(1u));
    let spacing = max(ddgi.volume_spacing.xyz, vec3<f32>(1.0e-4));
    let grid = (hit_pos - ddgi.volume_min.xyz) / spacing;
    if (any(grid < vec3<f32>(0.0)) || any(grid > vec3<f32>(counts - vec3<u32>(1u)))) {
        return vec3<f32>(0.0);
    }
    let g = clamp(vec3<u32>(round(grid)), vec3<u32>(0u), counts - vec3<u32>(1u));
    let idx = g.x + g.y * counts.x + g.z * counts.x * counts.y;
    if (probe_states[idx] == 0u) {
        return vec3<f32>(0.0);
    }
    let tile_texels = vec2<u32>(ddgi.probe_inner_size + 2u);
    let coord = ddgi_trace_atlas_coord(idx, n, tile_texels, ddgi.atlas_size.xy);
    return max(textureLoad(previous_irradiance, coord, 0).rgb, vec3<f32>(0.0));
}

fn probe_index_to_grid(index: u32) -> vec3<u32> {
    let nx = max(ddgi.probe_counts.x, 1u);
    let ny = max(ddgi.probe_counts.y, 1u);
    return vec3<u32>(index % nx, (index / nx) % ny, index / max(nx * ny, 1u));
}
fn update_probe_index(local_index: u32) -> u32 {
    return (u32(max(ddgi.camera_pos.w, 0.0)) + local_index) % max(ddgi.probe_counts.w, 1u);
}
fn probe_world_position(index: u32) -> vec3<f32> {
    let grid_pos = probe_index_to_grid(index);
    let g = vec3<f32>(f32(grid_pos.x), f32(grid_pos.y), f32(grid_pos.z));
    return ddgi.volume_min.xyz + g * ddgi.volume_spacing.xyz + probe_offsets[index].xyz;
}
fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}
fn spherical_fibonacci(i: u32, n: u32) -> vec3<f32> {
    let count = max(f32(n), 1.0);
    let phi = 2.39996322972865332 * f32(i);
    let z = 1.0 - (2.0 * (f32(i) + 0.5) / count);
    let r = sqrt(max(0.0, 1.0 - z * z));
    return vec3<f32>(cos(phi) * r, sin(phi) * r, z);
}
fn visible_to_light(pos: vec3<f32>, n: vec3<f32>, l: vec3<f32>) -> f32 {
    if (dot(n, l) <= 0.0) { return 0.0; }
    let h = trace_scene_limit(pos + n * max(params.min_ray_offset, T_EPS), l, min(params.max_rt_shadow_distance, params.rt_shadow_ray_t_max));
    return select(1.0, 0.0, h.hit != 0u);
}
fn sky_radiance(rd: vec3<f32>) -> vec3<f32> {
    let horizon = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    return params.skycolor.rgb * (0.35 + 0.65 * horizon) * max(0.0, 1.0 - params.sky_occlusion);
}
fn surface_radiance(hit: Hit, view_dir: vec3<f32>) -> vec3<f32> {
    let mat = materials[hit.material_index];
    var albedo = mat.baseColorFactor.rgb;
    if (mat.baseColorTex != 0u) {
        albedo = mat.baseColorFactor.rgb * textureSampleLevel(rt_material_texture, material_sampler, hit.uv, 0.0).rgb;
    }
    let l = normalize(-params.dir_light_dir.xyz);
    let direct = pbr_direct_light(PbrDirectLightInput(albedo, hit.normal, view_dir, l, params.dir_light_color.rgb * max(params.dir_light_dir.w, 0.0), mat.metallicFactor, mat.roughnessFactor, visible_to_light(hit.pos, hit.normal, l)));
    var emissive_texel = vec3<f32>(1.0);
    if (mat.material_flags1 != 0u) { emissive_texel = textureSampleLevel(rt_material_texture, material_sampler, hit.uv, 0.0).rgb; }
    let emissive = mat.emissiveFactor * mat.emissiveStrength * emissive_texel;

    // The old DDGI trace stored only direct/emissive radiance for hit surfaces.
    // In an enclosed GLTF scene most probe rays hit walls/floors, so if those
    // points are not directly lit the irradiance atlas becomes nearly black even
    // though RTGI still sees sky on its camera-facing hemisphere samples. Add a
    // conservative sky diffuse floor so probes receive stable incident light and
    // DDGI becomes visibly comparable to one-bounce RTGI instead of resolving to
    // zero. This is intentionally small; direct light and emissive still dominate.
    let sky_floor = albedo * params.skycolor.rgb * max(0.0, 1.0 - params.sky_occlusion) * (0.04 + 0.10 * clamp(hit.normal.y * 0.5 + 0.5, 0.0, 1.0));
    // Feed last frame's converged irradiance at the hit point back in as a
    // diffuse bounce (Lambertian: albedo/PI * incident irradiance). This is
    // what actually gives DDGI colored bounce light instead of a flat sky
    // tint; the sky_floor term above only stops probes from going fully
    // black in enclosed scenes, it does not carry any scene color.
    let indirect = sample_indirect_bounce(hit.pos, hit.normal) * albedo * (1.0 / PI);
    return max(direct + emissive + sky_floor + indirect, vec3<f32>(0.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (ddgi.enabled == 0u || id.x >= ddgi.rays_per_probe || id.y >= ddgi.probe_counts.w) { return; }
    let probe = update_probe_index(id.y);
    if (probe_states[probe] == 0u) {
        textureStore(ray_radiance_out, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(0.0));
        textureStore(ray_distance_out, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(ddgi.max_ray_distance, ddgi.max_ray_distance * ddgi.max_ray_distance, 0.0, 0.0));
        return;
    }
    // Keep the trace ray order exactly matched with the atlas update shaders.
    // ddgi_update_irradiance.comp.wgsl and ddgi_update_distance.comp.wgsl
    // reconstruct ray directions from only the ray index. If this pass jitters
    // directions by frame_number, the update pass integrates each radiance sample
    // into the wrong octahedral texel, so the DDGI atlas resolves as black/flat
    // even though the trace pass is running.
    let rd = normalize(spherical_fibonacci(id.x, ddgi.rays_per_probe));
    let origin = probe_world_position(probe);
    // Probe tracing should not use the screen-space DDGI view bias. A large
    // origin shift here skips nearby blockers and stores moments measured from
    // the wrong point, which shows up as light leaks, black patches, and flicker
    // in the resolve. Use only a tiny ray epsilon and compensate the stored hit
    // distance so distance moments remain measured from the probe position.
    let ray_bias = max(params.min_ray_offset, T_EPS);
    let hit = trace_scene_limit(origin + rd * ray_bias, rd, max(ddgi.max_ray_distance - ray_bias, ray_bias));
    var radiance = sky_radiance(rd);
    var dist = ddgi.max_ray_distance;
    if (hit.hit != 0u) {
        dist = min(hit.t + ray_bias, ddgi.max_ray_distance);
        radiance = surface_radiance(hit, rd);
    }
    textureStore(ray_radiance_out, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(radiance, 1.0));
    textureStore(ray_distance_out, vec2<i32>(i32(id.x), i32(id.y)), vec4<f32>(dist, dist * dist, 0.0, 0.0));
}
