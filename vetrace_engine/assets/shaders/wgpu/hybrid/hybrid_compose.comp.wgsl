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
// - gbuf_lightmap_uv rgba16float: xy = authored lightmap UV, z = validity mask, w = object index for editor outline.
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
@group(0) @binding(46) var rt_reflection_tex: texture_2d<f32>;
@group(0) @binding(47) var gbuf_lightmap_uv: texture_2d<f32>;


fn sample_raster_shadow_pcf(shadow_uv: vec2<f32>, compare_depth: f32, ndotl: f32) -> f32 {
    let dims = vec2<f32>(textureDimensions(raster_shadow_map));
    let texel = 1.0 / max(dims, vec2<f32>(1.0));
    // Wider filtering at grazing angles hides shadow-map texels without making
    // contact shadows vanish. This is still a single-map shadow, not CSM, so
    // very large shadow_max_distance values will remain lower-detail.
    let radius = mix(0.85, 1.85, 1.0 - clamp(ndotl, 0.0, 1.0));
    var sum = 0.0;
    var weight_sum = 0.0;
    for (var y: i32 = -2; y <= 2; y = y + 1) {
        for (var x: i32 = -2; x <= 2; x = x + 1) {
            let o = vec2<f32>(f32(x), f32(y));
            let w = 1.0 / (1.0 + dot(o, o) * 0.45);
            sum = sum + textureSampleCompareLevel(
                raster_shadow_map,
                raster_shadow_sampler,
                shadow_uv + o * texel * radius,
                compare_depth
            ) * w;
            weight_sum = weight_sum + w;
        }
    }
    return sum / max(weight_sum, 1.0e-5);
}

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
        textureStore(color_tex, px, vec4<f32>(sky, -1.0));
        return;
    }
    let object_id = textureLoad(gbuf_lightmap_uv, px, 0).w;

    let albedo = albedo_sample.rgb;
    let enc_n = textureLoad(gbuf_normal, px, 0).xyz;
    let n = normalize(enc_n * 2.0 - vec3<f32>(1.0));
    let gbuffer_material = decode_gbuffer_material(textureLoad(gbuf_material, px, 0));
    let metallic = gbuffer_material.metallic;
    let roughness = gbuffer_material.roughness;
    let emissive = albedo * gbuffer_material.emissive_luma;
    let light_dir = normalize(-params.dir_light_dir.xyz); // surface -> light
    let uv = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), depth01, 1.0);
    let world_h = params.inv_view_proj * clip;
    let world = world_h.xyz / max(world_h.w, 1e-6);
    let view_dir = normalize(params.camera_pos.xyz - world);
    let ndotl = dot(n, light_dir);

    // Receiver-side shadow bias. The shadow map stores the nearest surface from
    // the light, so the lookup only needs a small nudge toward the light to avoid
    // comparing a surface against its own exact depth. Keep these values small:
    // large normal/light offsets make contact shadows disappear when a cube is
    // close to, or touching, the receiver.
    let grazing = 1.0 - clamp(ndotl, 0.0, 1.0);
    let normal_bias = mix(0.0025, 0.012, grazing);
    let light_bias = mix(0.0010, 0.006, grazing);
    let depth_bias = mix(0.00035, 0.0015, grazing);
    let shadow_receiver_world = world + n * normal_bias + light_dir * light_bias;
    let shadow_clip = shadow_view_proj * vec4<f32>(shadow_receiver_world, 1.0);
    let shadow_ndc = shadow_clip.xyz / max(shadow_clip.w, 1e-6);
    let shadow_uv = shadow_ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    var raster_shadow = 1.0;
    if (ndotl > 0.001 && all(shadow_uv >= vec2<f32>(0.0)) && all(shadow_uv <= vec2<f32>(1.0)) && shadow_ndc.z >= 0.0 && shadow_ndc.z <= 1.0) {
        raster_shadow = sample_raster_shadow_pcf(shadow_uv, shadow_ndc.z - depth_bias, ndotl);
    }
    let shadow_factor = mix(0.45, 1.0, raster_shadow);
    if (params.rt_debug_view == 13u) {
        textureStore(color_tex, px, vec4<f32>(vec3<f32>(raster_shadow), object_id));
        return;
    }
    let direct = pbr_direct_light(PbrDirectLightInput(albedo, n, view_dir, light_dir, params.dir_light_color.xyz * params.dir_light_dir.w, metallic, roughness, shadow_factor));
    // AO is a single-channel visibility term.  Keep it scoped to indirect terms so
    // contact occlusion does not double-darken the direct-light shadowing path.
    let gi = textureLoad(gi_buffer, px, 0).rgb;
    let ao_visibility = clamp(textureLoad(ambient_occlusion_tex, px, 0).r, 0.0, 1.0);
    if (params.rt_debug_view == 5u) {
        textureStore(color_tex, px, vec4<f32>(vec3<f32>(ao_visibility), object_id));
        return;
    }
    if (params.rt_debug_view == 14u) {
        // High-contrast AO debug: black = strong occlusion, white = unoccluded.
        let contrast = pow(ao_visibility, 3.0);
        textureStore(color_tex, px, vec4<f32>(vec3<f32>(contrast), object_id));
        return;
    }
    let sky_irradiance = params.skycolor.rgb * max(0.18, 1.0 - params.sky_occlusion) * (0.22 + 0.12 * roughness) * ao_visibility;
    let ambient = pbr_ambient_diffuse(albedo, sky_irradiance + gi * ao_visibility, metallic);
    let fresnel = pbr_reflection_fresnel(albedo, n, view_dir, metallic);
    let ssr = textureLoad(ssr_reflection_tex, px, 0);
    let rt = textureLoad(rt_reflection_tex, px, 0);
    let ssr_color = max(ssr.rgb, vec3<f32>(0.0));
    let rt_color = max(rt.rgb, vec3<f32>(0.0));
    let ssr_confidence = clamp(ssr.a, 0.0, 1.0);
    if (params.rt_debug_view == 6u) {
        textureStore(color_tex, px, vec4<f32>(vec3<f32>(ssr_confidence), object_id));
        return;
    }
    if (params.rt_debug_view == 7u) {
        textureStore(color_tex, px, vec4<f32>(ssr_color, object_id));
        return;
    }
    if (params.rt_debug_view == 8u) {
        textureStore(color_tex, px, vec4<f32>(rt_color, object_id));
        return;
    }
    if (params.rt_debug_view == 9u) {
        textureStore(color_tex, px, vec4<f32>(gi, object_id));
        return;
    }
    if (params.rt_debug_view == 15u) {
        // Boosted GI debug so low-energy indirect lighting is visible while tuning.
        textureStore(color_tex, px, vec4<f32>(min(gi * 5.0, vec3<f32>(1.0)), object_id));
        return;
    }
    if (params.rt_debug_view == 11u) {
        let lm = textureLoad(gbuf_lightmap_uv, px, 0);
        let valid = clamp(lm.z, 0.0, 1.0);
        textureStore(color_tex, px, vec4<f32>(lm.xy * valid, valid, object_id));
        return;
    }
    let accurate_reflection = (gbuffer_material.feature_flags & 0x1u) != 0u;
    let reflective_feature = select(0.65, 1.0, accurate_reflection);
    let specular_strength = clamp(mix(0.35, 1.0, metallic) * reflective_feature, 0.0, 1.0);
    let smoothness = (1.0 - roughness) * (1.0 - roughness);
    let rt_confidence = clamp(rt.a, 0.0, 1.0) * (1.0 - roughness * 0.35) * specular_strength;
    // SSR is a screen-space approximation. It is allowed to improve rough/glossy
    // surfaces, but it must not overwrite a valid RT hit on mirrors/accurate
    // materials. Otherwise the final image shows correct RT reflection mixed with
    // broken SSR fragments from the camera-visible depth buffer only.
    let mirror_like = roughness <= 0.08 && metallic >= 0.75;
    let rt_should_own_pixel = rt_confidence > 0.02 && (accurate_reflection || mirror_like);
    let ssr_rt_suppression = select(max(0.0, 1.0 - rt_confidence * 0.90), 0.0, rt_should_own_pixel);
    let ssr_weight = ssr_confidence * smoothness * specular_strength * ssr_rt_suppression;
    let reflection_probe = mix(params.skycolor.rgb * fresnel, albedo * params.skycolor.rgb * 0.18, roughness);
    let ssr_or_probe = mix(reflection_probe, ssr_color, ssr_weight);
    let reflection_radiance = mix(ssr_or_probe, rt_color, rt_confidence);
    if (params.rt_debug_view == 10u) {
        textureStore(color_tex, px, vec4<f32>(reflection_radiance, object_id));
        return;
    }
    // Dielectric roughness=0 still has only ~4% physical F0, which made SSR look
    // invisible while debugging. Keep metallic physically strong, but add a small
    // mirror-visibility floor for extremely smooth non-metal materials so artists
    // can actually see SSR when roughness is set to 0.
    let mirror_visibility = (1.0 - smoothstep(0.02, 0.16, roughness)) * reflective_feature;
    let visible_fresnel = max(fresnel, vec3<f32>(0.04 + 0.18 * mirror_visibility * (1.0 - metallic)));
    let reflection_source = reflection_radiance * visible_fresnel * smoothness;
    let lit = emissive + direct + ambient + reflection_source;
    textureStore(color_tex, px, vec4<f32>(lit, object_id));
}

@compute @workgroup_size(8, 8, 1)
fn cloud_shadow_main(@builtin(global_invocation_id) id: vec3<u32>) {
    _ = id;
}
