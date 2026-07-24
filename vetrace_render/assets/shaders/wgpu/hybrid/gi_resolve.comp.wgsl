const GI_RESOLVE_METHOD_OFF: u32 = 0u;
const GI_RESOLVE_METHOD_BAKED_LIGHTMAP: u32 = 1u;
const GI_RESOLVE_METHOD_LIGHT_PROBES: u32 = 2u;
const GI_RESOLVE_METHOD_SDFGI: u32 = 3u;
const GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE: u32 = 4u;
const GI_RESOLVE_METHOD_DDGI: u32 = 6u;
const GI_RESOLVE_METHOD_SKY_IRRADIANCE_FALLBACK: u32 = 7u;

struct GiResolveParams {
    selected_method: u32,
    frame_number: u32,
    debug_flags: u32,
    _pad0: u32,
    temporal_blend: f32,
    baked_blend: f32,
    probe_blend: f32,
    sdfgi_blend: f32,
    rtgi_blend: f32,
    probe_count: u32,
    gi_resource_flags: u32,
    _pad1: vec2<u32>,
    fallback_irradiance: vec4<f32>,
    sdfgi_origin: vec4<f32>,
    sdfgi_extent_voxel: vec4<f32>,
    ddgi_origin_blend: vec4<f32>,
    ddgi_spacing_bias: vec4<f32>,
    ddgi_probe_counts_flags: vec4<u32>,
    ddgi_atlas_layout: vec4<u32>,
    ddgi_visibility: vec4<f32>,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(2) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(3) var lightmap_tex: texture_2d<f32>;
@group(0) @binding(4) var sdfgi_radiance: texture_3d<f32>;
@group(0) @binding(5) var rtgi_radiance: texture_2d<f32>;
@group(0) @binding(6) var gi_history: texture_2d<f32>;
@group(0) @binding(7) var gi_buffer: texture_storage_2d<rgba16float, write>;
@group(0) @binding(8) var<uniform> params: GiResolveParams;

struct LightProbe {
    position_radius: vec4<f32>,
    weight_visibility: vec4<f32>,
};

@group(0) @binding(9) var<storage, read> light_probes: array<LightProbe>;
@group(0) @binding(10) var<storage, read> light_probe_sh: array<vec4<f32>>;
@group(0) @binding(11) var gbuf_lightmap_uv: texture_2d<f32>;
@group(0) @binding(12) var ddgi_irradiance_atlas: texture_2d<f32>;
@group(0) @binding(13) var ddgi_distance_atlas: texture_2d<f32>;
@group(0) @binding(14) var<storage, read> ddgi_probe_state: array<u32>;
@group(0) @binding(15) var<storage, read> ddgi_relocation_offsets: array<vec4<f32>>;

const GI_RESOURCE_LIGHTMAP_ATLAS: u32 = 1u;
const GI_RESOURCE_LIGHTMAP_UVS: u32 = 2u;
const GI_RESOURCE_PROBES: u32 = 4u;
const GI_RESOURCE_SDFGI: u32 = 8u;
const GI_RESOURCE_DDGI: u32 = 16u;

const DDGI_DEBUG_VIEW_OFF: u32 = 0u;
const DDGI_DEBUG_VIEW_IRRADIANCE: u32 = 1u;
const DDGI_DEBUG_VIEW_DEPTH_VISIBILITY: u32 = 2u;
const DDGI_DEBUG_VIEW_PROBE_STATE: u32 = 3u;
const DDGI_DEBUG_VIEW_RELOCATION_OFFSETS: u32 = 4u;
const DDGI_DEBUG_VIEW_VOLUME_COORDINATES: u32 = 5u;
const DDGI_DEBUG_VIEW_ACTIVE_FALLBACK_STATUS: u32 = 6u;
const DDGI_DEBUG_VIEW_PROBE_TILE_INDEX: u32 = 7u;

fn resolved_surface(pixel: vec2<i32>) -> bool {
    return textureLoad(depth_tex, pixel, 0).r < 0.9999 && textureLoad(gbuf_albedo, pixel, 0).a > 0.0;
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(gbuf_normal, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn eval_probe_sh(probe_index: u32, n: vec3<f32>) -> vec3<f32> {
    let base = probe_index * 9u;
    let c0 = light_probe_sh[base + 0u].rgb;
    let c1 = light_probe_sh[base + 1u].rgb;
    let c2 = light_probe_sh[base + 2u].rgb;
    let c3 = light_probe_sh[base + 3u].rgb;
    // L0/L1 irradiance SH. Remaining coefficients are reserved for L2 producers.
    return max(c0 + c1 * n.y + c2 * n.z + c3 * n.x, vec3<f32>(0.0));
}

fn resolve_sky_irradiance_fallback(n: vec3<f32>) -> vec3<f32> {
    let horizon_wrap = 0.35 + 0.65 * clamp(n.y * 0.5 + 0.5, 0.0, 1.0);
    return max(params.fallback_irradiance.rgb * params.fallback_irradiance.w * horizon_wrap, vec3<f32>(0.0));
}

fn resolve_light_probe(world_pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    if ((params.gi_resource_flags & GI_RESOURCE_PROBES) == 0u || params.probe_count == 0u) {
        return resolve_sky_irradiance_fallback(n);
    }
    var sum = vec3<f32>(0.0);
    var wsum = 0.0;
    let count = min(params.probe_count, 256u);
    for (var i = 0u; i < count; i = i + 1u) {
        let p = light_probes[i];
        let to_probe = p.position_radius.xyz - world_pos;
        let dist = length(to_probe);
        let radius = max(p.position_radius.w, 1.0e-3);
        let falloff = max(1.0 - dist / radius, 0.0);
        let visibility = clamp(p.weight_visibility.x, 0.0, 1.0);
        let artist_weight = max(p.weight_visibility.y, 0.0);
        let w = falloff * falloff * visibility * max(artist_weight, 1.0e-4);
        sum = sum + eval_probe_sh(i, n) * w;
        wsum = wsum + w;
    }
    return sum / max(wsum, 1.0e-4);
}

fn resolve_baked_lightmap(pixel: vec2<i32>) -> vec3<f32> {
    if ((params.gi_resource_flags & (GI_RESOURCE_LIGHTMAP_ATLAS | GI_RESOURCE_LIGHTMAP_UVS)) != (GI_RESOURCE_LIGHTMAP_ATLAS | GI_RESOURCE_LIGHTMAP_UVS)) { return vec3<f32>(0.0); }
    let uvw = textureLoad(gbuf_lightmap_uv, pixel, 0);
    if (uvw.z <= 0.0) { return vec3<f32>(0.0); }
    let lm_dims = textureDimensions(lightmap_tex);
    let coord = clamp(vec2<i32>(uvw.xy * vec2<f32>(lm_dims)), vec2<i32>(0), vec2<i32>(lm_dims) - vec2<i32>(1));
    return textureLoad(lightmap_tex, coord, 0).rgb;
}

fn load_sdfgi_voxel(coord: vec3<i32>, dims: vec3<u32>) -> vec3<f32> {
    let c = clamp(coord, vec3<i32>(0), vec3<i32>(dims) - vec3<i32>(1));
    return clamp(textureLoad(sdfgi_radiance, c, 0).rgb, vec3<f32>(0.0), vec3<f32>(8.0));
}

fn resolve_sdfgi(world_pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    if ((params.gi_resource_flags & GI_RESOURCE_SDFGI) == 0u) { return vec3<f32>(0.0); }
    let sdf_dims = textureDimensions(sdfgi_radiance);
    let biased = world_pos + n * max(params.sdfgi_extent_voxel.w, 0.0);
    let local = (biased - params.sdfgi_origin.xyz) / max(params.sdfgi_extent_voxel.xyz, vec3<f32>(1.0e-4));
    if (any(local < vec3<f32>(0.0)) || any(local > vec3<f32>(1.0))) { return vec3<f32>(0.0); }

    // Manual trilinear filter. The old nearest-voxel lookup made SDFGI look
    // pixelated/blocky and exposed 3D volume slice boundaries.
    let dims_m1 = vec3<f32>(
        f32(max(sdf_dims.x, 1u) - 1u),
        f32(max(sdf_dims.y, 1u) - 1u),
        f32(max(sdf_dims.z, 1u) - 1u)
    );
    let voxel = local * dims_m1;
    let c0 = vec3<i32>(floor(voxel));
    let f = fract(voxel);
    let c000 = load_sdfgi_voxel(c0 + vec3<i32>(0, 0, 0), sdf_dims);
    let c100 = load_sdfgi_voxel(c0 + vec3<i32>(1, 0, 0), sdf_dims);
    let c010 = load_sdfgi_voxel(c0 + vec3<i32>(0, 1, 0), sdf_dims);
    let c110 = load_sdfgi_voxel(c0 + vec3<i32>(1, 1, 0), sdf_dims);
    let c001 = load_sdfgi_voxel(c0 + vec3<i32>(0, 0, 1), sdf_dims);
    let c101 = load_sdfgi_voxel(c0 + vec3<i32>(1, 0, 1), sdf_dims);
    let c011 = load_sdfgi_voxel(c0 + vec3<i32>(0, 1, 1), sdf_dims);
    let c111 = load_sdfgi_voxel(c0 + vec3<i32>(1, 1, 1), sdf_dims);
    let x00 = mix(c000, c100, f.x);
    let x10 = mix(c010, c110, f.x);
    let x01 = mix(c001, c101, f.x);
    let x11 = mix(c011, c111, f.x);
    let y0 = mix(x00, x10, f.y);
    let y1 = mix(x01, x11, f.y);
    return mix(y0, y1, f.z);
}


fn oct_wrap(v: vec2<f32>) -> vec2<f32> {
    return (vec2<f32>(1.0) - abs(v.yx)) * select(vec2<f32>(-1.0), vec2<f32>(1.0), v >= vec2<f32>(0.0));
}

fn oct_encode(n_in: vec3<f32>) -> vec2<f32> {
    var n = n_in / max(abs(n_in.x) + abs(n_in.y) + abs(n_in.z), 1.0e-6);
    if (n.z < 0.0) {
        let wrapped = oct_wrap(n.xy);
        n = vec3<f32>(wrapped, n.z);
    }
    return n.xy * 0.5 + vec2<f32>(0.5);
}

fn ddgi_probe_index(c: vec3<u32>, counts: vec3<u32>) -> u32 {
    return c.x + c.y * counts.x + c.z * counts.x * counts.y;
}

fn ddgi_atlas_coord(probe_index: u32, dir: vec3<f32>, tile_texels: vec2<u32>, atlas_dims: vec2<u32>) -> vec2<i32> {
    // tile_texels includes a 1-texel border. The resolved sample should address
    // the inner DDGI texels; sampling the border directly folds unrelated wrap
    // texels into the main lookup and creates blocky/glitchy probe transitions.
    let safe_tile = max(tile_texels, vec2<u32>(3u));
    let inner = max(safe_tile - vec2<u32>(2u), vec2<u32>(1u));
    let probes_per_row = max(atlas_dims.x / max(safe_tile.x, 1u), 1u);
    let tile = vec2<u32>(probe_index % probes_per_row, probe_index / probes_per_row);
    let oct = oct_encode(normalize(dir));
    let texel = vec2<u32>(clamp(oct * vec2<f32>(inner - vec2<u32>(1u)), vec2<f32>(0.0), vec2<f32>(inner - vec2<u32>(1u))));
    let coord = min(tile * safe_tile + texel + vec2<u32>(1u), atlas_dims - vec2<u32>(1u));
    return vec2<i32>(i32(coord.x), i32(coord.y));
}

fn ddgi_sample_probe(probe_index: u32, probe_pos: vec3<f32>, world_pos: vec3<f32>, n: vec3<f32>, cell_weight: f32) -> vec4<f32> {
    let state = ddgi_probe_state[probe_index];
    let probe_is_enabled = select(0.0, 1.0, state != 0u);
    if (probe_is_enabled <= 0.0 || cell_weight <= 0.0) { return vec4<f32>(0.0); }
    let to_surface = world_pos - probe_pos;
    let dist = length(to_surface);
    let dir = select(to_surface / max(dist, 1.0e-4), n, dist <= 1.0e-4);
    let irr_dims = textureDimensions(ddgi_irradiance_atlas);
    let dist_dims = textureDimensions(ddgi_distance_atlas);
    let irr_texels = max(params.ddgi_atlas_layout.xy, vec2<u32>(1u));
    let dist_texels = max(params.ddgi_atlas_layout.zw, vec2<u32>(1u));
    let irradiance = textureLoad(ddgi_irradiance_atlas, ddgi_atlas_coord(probe_index, n, irr_texels, irr_dims), 0).rgb;
    let moments = textureLoad(ddgi_distance_atlas, ddgi_atlas_coord(probe_index, dir, dist_texels, dist_dims), 0).rg;
    let mean_dist = max(moments.x, 1.0e-4);
    let variance = max(moments.y - mean_dist * mean_dist, params.ddgi_visibility.z);
    let cheby = variance / (variance + max(dist - mean_dist, 0.0) * max(dist - mean_dist, 0.0));
    let visibility = mix(1.0, clamp(cheby, 0.0, 1.0), clamp(params.ddgi_visibility.y, 0.0, 1.0));
    let facing = pow(max(dot(n, normalize(probe_pos - world_pos)), 0.0), max(params.ddgi_visibility.x, 0.0));
    let weight = cell_weight * probe_is_enabled * visibility * max(facing, 0.05);
    return vec4<f32>(max(irradiance, vec3<f32>(0.0)) * weight, weight);
}


fn debug_tonemap(v: vec3<f32>) -> vec3<f32> {
    return v / (v + vec3<f32>(1.0));
}

fn ddgi_debug_color(world_pos: vec3<f32>, n: vec3<f32>, ddgi_debug_view: u32) -> vec3<f32> {
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_OFF) {
        return vec3<f32>(0.0);
    }
    if ((params.gi_resource_flags & GI_RESOURCE_DDGI) == 0u) {
        return select(vec3<f32>(1.0, 0.12, 0.05), vec3<f32>(1.0, 0.72, 0.05), ddgi_debug_view == DDGI_DEBUG_VIEW_ACTIVE_FALLBACK_STATUS);
    }

    let counts = max(params.ddgi_probe_counts_flags.xyz, vec3<u32>(1u));
    let spacing = max(params.ddgi_spacing_bias.xyz, vec3<f32>(1.0e-4));
    // Do not derive camera position from inv_view_proj[3]. That column is not a
    // stable world-space camera position for perspective matrices, so using it
    // as a view direction makes DDGI pop or fall outside the volume. Until the
    // resolve uniform carries camera_pos explicitly, keep resolve bias normal-only.
    let biased_world = world_pos + n * max(params.ddgi_spacing_bias.w, 0.0);
    let grid = (biased_world - params.ddgi_origin_blend.xyz) / spacing;
    let inside = !(any(grid < vec3<f32>(0.0)) || any(grid > vec3<f32>(counts - vec3<u32>(1u))));
    if (!inside) {
        return select(vec3<f32>(0.45, 0.0, 0.85), vec3<f32>(1.0, 0.72, 0.05), ddgi_debug_view == DDGI_DEBUG_VIEW_ACTIVE_FALLBACK_STATUS);
    }

    let base = clamp(vec3<u32>(floor(grid)), vec3<u32>(0u), counts - vec3<u32>(1u));
    let probe_index = ddgi_probe_index(base, counts);
    let probe_pos = params.ddgi_origin_blend.xyz + vec3<f32>(base) * spacing + ddgi_relocation_offsets[probe_index].xyz;
    let to_surface = biased_world - probe_pos;
    let probe_dist = length(to_surface);
    let probe_dir = select(to_surface / max(probe_dist, 1.0e-4), n, probe_dist <= 1.0e-4);
    let state = ddgi_probe_state[probe_index];
    let probe_enabled = state != 0u;
    let irr_dims = textureDimensions(ddgi_irradiance_atlas);
    let dist_dims = textureDimensions(ddgi_distance_atlas);
    let irr_texels = max(params.ddgi_atlas_layout.xy, vec2<u32>(1u));
    let dist_texels = max(params.ddgi_atlas_layout.zw, vec2<u32>(1u));

    if (ddgi_debug_view == DDGI_DEBUG_VIEW_IRRADIANCE) {
        let irradiance = textureLoad(ddgi_irradiance_atlas, ddgi_atlas_coord(probe_index, n, irr_texels, irr_dims), 0).rgb;
        return debug_tonemap(max(irradiance, vec3<f32>(0.0)) * 2.0);
    }
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_DEPTH_VISIBILITY) {
        let moments = textureLoad(ddgi_distance_atlas, ddgi_atlas_coord(probe_index, probe_dir, dist_texels, dist_dims), 0).rg;
        let mean_dist = max(moments.x, 1.0e-4);
        let variance = max(moments.y - mean_dist * mean_dist, params.ddgi_visibility.z);
        let delta = max(probe_dist - mean_dist, 0.0);
        let visibility = variance / (variance + delta * delta);
        let range = max(length(spacing), 1.0e-4);
        return vec3<f32>(clamp(visibility, 0.0, 1.0), clamp(mean_dist / range, 0.0, 1.0), clamp(variance / (range * range), 0.0, 1.0));
    }
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_PROBE_STATE) {
        return select(vec3<f32>(1.0, 0.05, 0.02), vec3<f32>(0.05, 1.0, 0.12), probe_enabled);
    }
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_RELOCATION_OFFSETS) {
        let offset = ddgi_relocation_offsets[probe_index].xyz / spacing;
        return clamp(offset * 0.5 + vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_VOLUME_COORDINATES) {
        return fract(max(grid, vec3<f32>(0.0)));
    }
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_ACTIVE_FALLBACK_STATUS) {
        return select(vec3<f32>(1.0, 0.12, 0.05), vec3<f32>(0.05, 1.0, 0.12), probe_enabled);
    }
    if (ddgi_debug_view == DDGI_DEBUG_VIEW_PROBE_TILE_INDEX) {
        let nx = max(counts.x, 1u);
        let ny = max(counts.y, 1u);
        let total = max(counts.x * counts.y * counts.z, 1u);
        return vec3<f32>(
            f32(probe_index % nx) / f32(nx),
            f32((probe_index / nx) % ny) / f32(ny),
            f32(probe_index) / f32(total)
        );
    }
    return vec3<f32>(0.0);
}

fn resolve_ddgi(world_pos: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    if ((params.gi_resource_flags & GI_RESOURCE_DDGI) == 0u) {
        return resolve_light_probe(world_pos, n);
    }
    let counts = max(params.ddgi_probe_counts_flags.xyz, vec3<u32>(1u));
    let spacing = max(params.ddgi_spacing_bias.xyz, vec3<f32>(1.0e-4));
    // Do not derive camera position from inv_view_proj[3]. That column is not a
    // stable world-space camera position for perspective matrices, so using it
    // as a view direction makes DDGI pop or fall outside the volume. Until the
    // resolve uniform carries camera_pos explicitly, keep resolve bias normal-only.
    let biased_world = world_pos + n * max(params.ddgi_spacing_bias.w, 0.0);
    let grid = (biased_world - params.ddgi_origin_blend.xyz) / spacing;
    if (any(grid < vec3<f32>(0.0)) || any(grid > vec3<f32>(counts - vec3<u32>(1u)))) {
        return resolve_light_probe(world_pos, n);
    }
    let basef = floor(grid);
    let fracv = fract(grid);
    let base = vec3<u32>(clamp(basef, vec3<f32>(0.0), vec3<f32>(counts - vec3<u32>(1u))));
    var sum = vec3<f32>(0.0);
    var wsum = 0.0;
    for (var oz = 0u; oz <= 1u; oz = oz + 1u) {
      for (var oy = 0u; oy <= 1u; oy = oy + 1u) {
        for (var ox = 0u; ox <= 1u; ox = ox + 1u) {
          let o = vec3<u32>(ox, oy, oz);
          let c = min(base + o, counts - vec3<u32>(1u));
          let cw = mix(1.0 - fracv.x, fracv.x, f32(ox)) * mix(1.0 - fracv.y, fracv.y, f32(oy)) * mix(1.0 - fracv.z, fracv.z, f32(oz));
          let idx = ddgi_probe_index(c, counts);
          let probe_pos = params.ddgi_origin_blend.xyz + vec3<f32>(c) * spacing + ddgi_relocation_offsets[idx].xyz;
          let s = ddgi_sample_probe(idx, probe_pos, biased_world, n, cw);
          sum = sum + s.rgb;
          wsum = wsum + s.a;
        }
      }
    }
    if (wsum <= 1.0e-4) { return resolve_light_probe(world_pos, n); }
    return mix(resolve_light_probe(world_pos, n), sum / wsum, clamp(params.ddgi_visibility.w, 0.0, 1.0));
}

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip_xy = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var world = params.inv_view_proj * vec4<f32>(clip_xy, depth, 1.0);
    return (world / world.w).xyz;
}

fn load_rtgi_denoised(pixel: vec2<i32>, dims: vec2<u32>) -> vec3<f32> {
    let center_n = unpack_normal(pixel);
    let center_depth = textureLoad(depth_tex, pixel, 0).r;
    var center = clamp(textureLoad(rtgi_radiance, pixel, 0).rgb, vec3<f32>(0.0), vec3<f32>(16.0));
    var sum = center * 1.5;
    var wsum = 1.5;

    // 5x5 bilateral resolve. RTGI is only 1 spp, so the resolve must carry
    // more of the smoothing than the raw ray pass. Depth is compared in raw
    // depth space but with a softer threshold than before so close surfaces do
    // not become salt-and-pepper noise.
    for (var oy = -2; oy <= 2; oy = oy + 1) {
        for (var ox = -2; ox <= 2; ox = ox + 1) {
            if (ox == 0 && oy == 0) { continue; }
            let q = pixel + vec2<i32>(ox, oy);
            if (q.x < 0 || q.y < 0 || q.x >= i32(dims.x) || q.y >= i32(dims.y) || !resolved_surface(q)) { continue; }
            let ndot = max(dot(center_n, unpack_normal(q)), 0.0);
            let d = abs(textureLoad(depth_tex, q, 0).r - center_depth);
            let spatial = exp(-f32(ox * ox + oy * oy) * 0.32);
            let depth_w = exp(-d * 48.0);
            let normal_w = ndot * ndot;
            let w = spatial * depth_w * normal_w;
            let v = clamp(textureLoad(rtgi_radiance, q, 0).rgb, vec3<f32>(0.0), vec3<f32>(16.0));
            sum = sum + v * w;
            wsum = wsum + w;
        }
    }
    return sum / max(wsum, 1.0e-4);
}

fn reproject_history(pixel: vec2<i32>, dims: vec2<u32>) -> vec3<f32> {
    let world = reconstruct_world(pixel, dims, textureLoad(depth_tex, pixel, 0).r);
    let prev_clip = params.prev_view_proj * vec4<f32>(world, 1.0);
    let prev_ndc = prev_clip.xyz / max(prev_clip.w, 1.0e-5);
    let prev_uv = prev_ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    let prev_px = vec2<i32>(prev_uv * vec2<f32>(dims));
    if (prev_px.x < 0 || prev_px.y < 0 || prev_px.x >= i32(dims.x) || prev_px.y >= i32(dims.y)) { return textureLoad(gi_history, pixel, 0).rgb; }
    return textureLoad(gi_history, prev_px, 0).rgb;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (!resolved_surface(pixel) || params.selected_method == GI_RESOLVE_METHOD_OFF) {
        textureStore(gi_buffer, pixel, vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    let world = reconstruct_world(pixel, dims, textureLoad(depth_tex, pixel, 0).r);
    let n = unpack_normal(pixel);
    var gi = vec3<f32>(0.0);
    if (params.selected_method == GI_RESOLVE_METHOD_BAKED_LIGHTMAP) {
        gi = resolve_baked_lightmap(pixel) * params.baked_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_LIGHT_PROBES) {
        gi = resolve_light_probe(world, n) * params.probe_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_SDFGI) {
        gi = resolve_sdfgi(world, n) * params.sdfgi_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE) {
        gi = load_rtgi_denoised(pixel, dims) * params.rtgi_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_DDGI) {
        gi = resolve_ddgi(world, n);
    } else if (params.selected_method == GI_RESOLVE_METHOD_SKY_IRRADIANCE_FALLBACK) {
        gi = resolve_sky_irradiance_fallback(n);
    }

    let ddgi_debug_view = (params.debug_flags >> 8u) & 0xffu;
    if (params.selected_method == GI_RESOLVE_METHOD_DDGI && ddgi_debug_view != DDGI_DEBUG_VIEW_OFF) {
        gi = ddgi_debug_color(world, n, ddgi_debug_view);
    }

    if (ddgi_debug_view == DDGI_DEBUG_VIEW_OFF && params.frame_number > 0u && params.temporal_blend > 0.0) {
        let history = reproject_history(pixel, dims);
        // Stronger but safer temporal filter for 1 spp RTGI. Clamp history near
        // the current luminance so it smooths noise without dragging old bright
        // colors across newly visible geometry.
        let range = max(vec3<f32>(0.08), max(gi, history) * 0.45 + vec3<f32>(0.05));
        let lo = max(vec3<f32>(0.0), gi - range);
        let hi = gi + range;
        gi = mix(gi, clamp(history, lo, hi), clamp(params.temporal_blend, 0.0, 0.92));
    }
    textureStore(gi_buffer, pixel, vec4<f32>(max(gi, vec3<f32>(0.0)), 1.0));
}
