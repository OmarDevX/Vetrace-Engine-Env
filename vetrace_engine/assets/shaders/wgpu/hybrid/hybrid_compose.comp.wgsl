// Shared with pathtrace.comp.wgsl by Rust concat! during shader module creation.
struct Params {
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_up: vec4<f32>,
    camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>,
    fov: f32,
    num_objects: i32,
    is_fisheye: i32,
    _pad0: i32,
    skycolor: vec4<f32>,
    taa_jitter: vec2<f32>,
    current_time: f32,
    frame_number: i32,
    selected_index: i32,
    max_bounces: i32,
    light_samples: i32,
    dir_shadow_samples: i32,
    shadow_mode: u32,
    raytraced_shadows_enabled: u32,
    shadow_quality: u32,
    max_shadow_rays: u32,
    emissive_shadow_samples: u32,
    directional_shadow_samples: u32,
    cloud_object_shadows_enabled: u32,
    max_rt_shadow_distance: f32,
    rt_shadow_ray_t_max: f32,
    min_soft_shadow_radius: f32,
    raytraced_reflections_enabled: u32,
    _pad_reflections: u32,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    sky_occlusion: f32,
    total_triangles: u32,
    total_bvh_nodes: u32,
    total_tri_bvh_nodes: u32,
    dof_aperture: f32,
    dof_focus_dist: f32,
    dof_enable: u32,
    _pad_dof: u32,
    atmosphere: u32,
    atmo_count: u32,
    cloud_count: u32,
    atmosphere_mode: u32,
    atmosphere_sun_controls: vec4<f32>,
    cloud_history_weight: f32,
    cloud_sample_count: u32,
    cloud_temporal_quality: u32,
    cloud_shadow_mode: u32,
    renderer_mode: u32,
    rt_debug_view: u32,
};


// Shared raster G-buffer contract (primitive + mesh passes; produced by primitive_gbuffer.wgsl and simple_pbr.wgsl):
// - gbuf_albedo rgba8unorm: rgb = linear base color, a = coverage/valid surface mask.
// - gbuf_normal rgba16float: xyz = world-space normal encoded as normal * 0.5 + 0.5, w = reserved (1.0).
// - gbuf_material rgba8uint: x = metallic UNORM8, y = roughness UNORM8, z = emissive luma UNORM8,
//   w = packed metadata; low nibble = feature flags, high nibble = object/material ID bucket.
// - depth texture r32float: device depth used for world-position reconstruction and sky rejection.
const GBUFFER_FEATURE_FLAGS_MASK: u32 = 0x0fu;
const GBUFFER_ID_SHIFT: u32 = 4u;
const GBUFFER_ID_MASK: u32 = 0xf0u;

struct GBufferMaterial {
    metallic: f32,
    roughness: f32,
    emissive_luma: f32,
    feature_flags: u32,
    id_bucket: u32,
};

fn decode_gbuffer_unorm8(v: u32) -> f32 {
    return f32(v) / 255.0;
}

fn decode_gbuffer_material(material: vec4<u32>) -> GBufferMaterial {
    return GBufferMaterial(
        decode_gbuffer_unorm8(material.x),
        max(decode_gbuffer_unorm8(material.y), 0.04),
        decode_gbuffer_unorm8(material.z),
        material.w & GBUFFER_FEATURE_FLAGS_MASK,
        (material.w & GBUFFER_ID_MASK) >> GBUFFER_ID_SHIFT,
    );
}

@group(0) @binding(4) var<uniform> params: Params;
@group(0) @binding(5) var color_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(6) var depth_tex: texture_storage_2d<r32float, read_write>;
@group(0) @binding(8) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(9) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(10) var gbuf_material: texture_2d<u32>;
@group(0) @binding(40) var<uniform> shadow_view_proj: mat4x4<f32>;
@group(0) @binding(41) var raster_shadow_map: texture_depth_2d;
@group(0) @binding(42) var raster_shadow_sampler: sampler_comparison;
@group(0) @binding(43) var gi_buffer: texture_2d<f32>;
@group(0) @binding(44) var ambient_occlusion_tex: texture_2d<f32>;
@group(0) @binding(45) var ssr_reflection_tex: texture_2d<f32>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(color_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let px = vec2<i32>(id.xy);
    let depth01 = textureLoad(depth_tex, px).r;
    let albedo_sample = textureLoad(gbuf_albedo, px, 0);
    if (depth01 >= 0.9999 || albedo_sample.a <= 0.0) {
        let uv = vec2<f32>(f32(id.x) / max(f32(dims.x), 1.0), f32(id.y) / max(f32(dims.y), 1.0));
        let sky = params.skycolor.xyz * (0.55 + 0.45 * (1.0 - uv.y));
        textureStore(color_tex, px, vec4<f32>(sky, 1.0));
        return;
    }

    let albedo = albedo_sample.rgb;
    let enc_n = textureLoad(gbuf_normal, px, 0).xyz;
    let n = normalize(enc_n * 2.0 - vec3<f32>(1.0));
    let gbuffer_material = decode_gbuffer_material(textureLoad(gbuf_material, px, 0));
    let metallic = gbuffer_material.metallic;
    let roughness = gbuffer_material.roughness;
    let emissive = albedo * gbuffer_material.emissive_luma;
    let light_dir = normalize(-params.dir_light_dir.xyz);
    let uv = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), depth01, 1.0);
    let world_h = params.inv_view_proj * clip;
    let world = world_h.xyz / max(world_h.w, 1e-6);
    let view_dir = normalize(world - params.camera_pos.xyz);
    let shadow_clip = shadow_view_proj * vec4<f32>(world + n * 0.03, 1.0);
    let shadow_ndc = shadow_clip.xyz / max(shadow_clip.w, 1e-6);
    let shadow_uv = shadow_ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    var raster_shadow = 1.0;
    if (all(shadow_uv >= vec2<f32>(0.0)) && all(shadow_uv <= vec2<f32>(1.0)) && shadow_ndc.z >= 0.0 && shadow_ndc.z <= 1.0) {
        raster_shadow = textureSampleCompareLevel(raster_shadow_map, raster_shadow_sampler, shadow_uv, shadow_ndc.z - 0.0015);
    }
    let shadow_factor = mix(0.25, 1.0, raster_shadow);
    let direct = pbr_direct_light(PbrDirectLightInput(albedo, n, view_dir, light_dir, params.dir_light_color.xyz * params.dir_light_dir.w, metallic, roughness, shadow_factor));
    let gi = textureLoad(gi_buffer, px, 0).rgb;
    let ao = clamp(textureLoad(ambient_occlusion_tex, px, 0).r, 0.0, 1.0);
    if (params.rt_debug_view == 5u) {
        textureStore(color_tex, px, vec4<f32>(vec3<f32>(ao), 1.0));
        return;
    }
    let sky_irradiance = params.skycolor.rgb * max(0.03, 1.0 - params.sky_occlusion) * (0.12 + 0.08 * roughness);
    let ambient = pbr_ambient_diffuse(albedo, (sky_irradiance + gi) * ao, metallic);
    let fresnel = pbr_reflection_fresnel(albedo, n, view_dir, metallic);
    let ssr = textureLoad(ssr_reflection_tex, px, 0);
    let ssr_conf = clamp(ssr.a, 0.0, 1.0) * (1.0 - roughness * 0.65);
    let reflection_probe = mix(params.skycolor.rgb * fresnel, albedo * params.skycolor.rgb * 0.18, roughness) * select(0.35, 1.0, params.raytraced_reflections_enabled != 0u);
    let reflection_source = mix(reflection_probe, ssr.rgb * fresnel, ssr_conf);
    let lit = emissive + direct + ambient + reflection_source;
    textureStore(color_tex, px, vec4<f32>(lit, 1.0));
}

@compute @workgroup_size(8, 8, 1)
fn cloud_shadow_main(@builtin(global_invocation_id) id: vec3<u32>) {
    _ = id;
}
