// EXPERIMENTAL/FUTURE: moved out of the active hybrid shader directory because Rust does not wire this shader into a pipeline yet. See docs/SHADER_ARCHITECTURE.md.
// WGSL path tracer + thin-lens DOF (optimized atmosphere)

// -----------------------------
// Scene/Material structs
// -----------------------------
struct Object {
    orientation: vec4<f32>,
    position: vec3<f32>, _pad1: f32,
    size: vec3<f32>, _pad2: f32,
    scale: vec3<f32>, _pad2b: f32,
    material_index: u32,
    radius: f32,
    is_cube: u32,
    is_glass: u32,
    is_mesh: u32,
    triangle_start_idx: u32,
    triangle_count: u32,
    tri_bvh_start: u32,
    tri_bvh_count: u32,
    is_shaded: u32,
    casts_raster_shadow: u32,
    casts_raytraced_shadow: u32,
    shadow_importance: f32,
    max_shadow_distance: f32,
    scene_flags: u32,
    gi_flags: u32,
    _gi_pad0: u32,
    _gi_pad1: u32,
};

struct Triangle {
    v0: vec3<f32>, _pad0: f32,
    e1: vec3<f32>, _pad1: f32,
    e2: vec3<f32>, _pad2: f32,
    n0: vec3<f32>, _pad3: f32,
    n1: vec3<f32>, _pad4: f32,
    n2: vec3<f32>, _pad5: f32,
    uv0: vec2<f32>, duv1: vec2<f32>,
    duv2: vec2<f32>, material_index: u32, _pad6: u32,
};

struct BvhNode {
    bmin: vec4<f32>,
    bmax: vec4<f32>,
    child_object: vec4<i32>,
};

struct TriBvhNode {
    bmin: vec4<f32>,
    bmax: vec4<f32>,
    child_tri: vec4<i32>,
};

// Must match Rust: vetrace_engine/src/scene/object.rs::GpuMaterial
struct MaterialParams {
    baseColorFactor: vec4<f32>,
    emissiveFactor: vec3<f32>, emissiveStrength: f32,
    metallicFactor: f32,
    roughnessFactor: f32,
    ior: f32,
    baseColorTex: u32,
    f0: vec3<f32>, has_custom_material: u32,
    custom_material_id: u32,
    material_flags0: u32,
    material_flags1: u32,
    material_flags2: u32,
    material_flags3: u32,
    material_flags4: u32,
    material_flags5: u32,
    material_flags6: u32,
};

struct CustomMaterialParams {
    color_tint: vec4<f32>,
    base_props: vec4<f32>,            // roughness, metallic, noise_scale, emission_strength
    custom_floats: vec4<f32>,         // custom_float_1..4
    transparency_params: vec4<f32>,   // transparency, transmission, transmission_roughness, refraction_ior
    subsurface_params: vec4<f32>,     // subsurface_strength, subsurface_radius.rgb
    coat_aniso: vec4<f32>,            // clearcoat_strength, clearcoat_roughness, anisotropy, anisotropy_rotation
    sheen_params: vec4<f32>,          // sheen_strength, sheen_tint.rgb
    normal_disp: vec4<f32>,           // normal_strength, displacement_strength, unused0, unused1
    texture_index: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

struct MaterialResult {
    base_color: vec3<f32>,
    normal: vec3<f32>,
    roughness: f32,
    metallic: f32,
    emission: vec3<f32>,
    // Transparency and extended outputs
    transparency: f32,
    transmission: f32,
    transmission_roughness: f32,
    ior: f32,
    subsurface: vec4<f32>,     // strength + RGB radii
    clearcoat: vec2<f32>,      // strength + roughness
    anisotropy: vec2<f32>,     // strength + rotation
    sheen: vec4<f32>,          // strength + RGB tint
    displacement: f32,
};

// MATERIAL_FUNCTIONS_PLACEHOLDER

struct Atmosphere {
    center_radius: vec4<f32>,
    atmo_g_height: vec4<f32>,
    ray_beta: vec4<f32>,
    mie_beta: vec4<f32>,
    ambient_beta: vec4<f32>,
    absorption_beta: vec4<f32>,
    absorb_params: vec4<f32>,
    ozone_params: vec4<f32>,
    absorption_layer_params: vec4<f32>,
    multi_scatter_params: vec4<f32>,
};

struct Scattering {
    color: vec3<f32>,
    transmittance: vec3<f32>,
};

const MAX_ATMOSPHERES: u32 = 8u;
const MAX_VOLUMETRIC_CLOUDS: u32 = 8u;

struct VolumetricCloud {
    center_base_thickness: vec4<f32>, // xyz = planet center, w = cloud base radius
    coverage_density_noise_phase: vec4<f32>,
    wind_steps: vec4<f32>,
    light_padding: vec4<f32>,
    multi_scatter: vec4<f32>,
    shape_params: vec4<f32>, // x=thickness, y=primary steps, z=shape scale, w=detail scale
    weather_params: vec4<f32>, // x=weather scale, yz=weather offset, w=macro variation
    detail_params: vec4<f32>, // x=erosion, y=cloud type, z=anvil/top shape, w=curl strength
    lighting_params0: vec4<f32>, // x=forward g, y=backward g, z=forward lobe blend, w=powder strength
    lighting_params1: vec4<f32>, // x=silver lining strength, yzw=reserved
};

// --- runtime safety caps ---
const MAX_TLAS_ITERS          : u32 = 8u;
const DEFAULT_MAX_TRAVERSAL_STEPS : u32 = 512u;
const DEFAULT_MAX_TRANSPARENT_SURFACES : u32 = 8u;
const DEFAULT_MIN_RAY_OFFSET : f32 = 0.01;
const MAX_SCATTER_STEPS       : i32 = 24;
const MAX_LIGHT_STEPS         : i32 = 6;
const MAX_ATMO_LIGHTS         : u32 = 2u;
const MAX_LIGHT_SAMPLES       : u32 = 8u;
const MAX_IMPORTANT_PIXEL_LIGHTS : u32 = 4u;
const MAX_DIR_SHADOW_SAMPLES  : u32 = 8u;
const SHADOW_QUALITY_OFF      : u32 = 0u;
const SHADOW_QUALITY_LOW      : u32 = 1u;
const SHADOW_QUALITY_MEDIUM   : u32 = 2u;
const SHADOW_QUALITY_HIGH     : u32 = 3u;
const SHADOW_MODE_NONE        : u32 = 0u;
const SHADOW_MODE_RASTER      : u32 = 1u;
const SHADOW_MODE_RT_HARD     : u32 = 2u;
const SHADOW_MODE_RT_SOFT     : u32 = 3u;
const SHADOW_MODE_HYBRID      : u32 = 4u;
const T_EARLY_OUT             : f32 = 1e-3;

// Must match Rust: vetrace_engine/src/rendering/wgpu_renderer/types.rs::ShaderParams
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
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    sky_occlusion: f32,
    total_triangles: u32,
    total_bvh_nodes: u32,
    total_tri_bvh_nodes: u32,

    // ---- DOF (Thin Lens) ----
    dof_aperture: f32,      // lens diameter in world units (0 = off)
    dof_focus_dist: f32,    // distance along +camera_front to focus plane
    dof_enable: u32,        // 0 off / 1 on
    _pad_dof: u32,
    atmosphere: u32,
    atmo_count: u32,
    cloud_count: u32,
    atmosphere_mode: u32, // 0 = LUT atmosphere, 1 = inline/debug atmosphere
    atmosphere_sun_controls: vec4<f32>,
    cloud_history_weight: f32,
    cloud_sample_count: u32,
    cloud_temporal_quality: u32,
    cloud_shadow_mode: u32,
    renderer_mode: u32,
    rt_debug_view: u32,
    rt_debug_counters: u32,
    max_traversal_steps: u32,
    max_transparent_surfaces: u32,
    shadow_max_distance: f32,
    reflection_max_distance: f32,
    gi_max_distance: f32,
    min_ray_offset: f32,
    atmos: array<Atmosphere, MAX_ATMOSPHERES>,
};


const RENDERER_MODE_RASTER_GAME: u32 = 0u;
const RENDERER_MODE_HYBRID_EFFECTS: u32 = 1u;
const RENDERER_MODE_PATH_TRACE_PREVIEW: u32 = 2u;
const RENDERER_MODE_CINEMATIC_PATH_TRACE: u32 = 3u;

fn uses_path_traced_primary_visibility() -> bool {
    return params.renderer_mode == RENDERER_MODE_PATH_TRACE_PREVIEW || params.renderer_mode == RENDERER_MODE_CINEMATIC_PATH_TRACE;
}

// -----------------------------
// Bindings
// -----------------------------
@group(0) @binding(0)  var<storage, read> objects: array<Object>;
@group(0) @binding(1)  var<storage, read> triangles: array<Triangle>;
@group(0) @binding(2)  var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(3)  var<storage, read> tri_bvh_nodes: array<TriBvhNode>;
@group(0) @binding(4)  var<uniform> params: Params;
@group(0) @binding(5)  var color_tex:  texture_storage_2d<rgba16float, write>;
@group(0) @binding(6)  var depth_tex:  texture_storage_2d<r32float, read_write>;
@group(0) @binding(7)  var normal_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(8)  var gbuf_albedo:   texture_2d<f32>;
@group(0) @binding(9)  var gbuf_normal:   texture_2d<f32>;
@group(0) @binding(10) var gbuf_material: texture_2d<u32>;
@group(0) @binding(17) var<storage, read> materials: array<MaterialParams>;
@group(0) @binding(21) var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(22) var tex_sampler: sampler;
@group(0) @binding(23) var<storage, read> custom_materials: array<CustomMaterialParams>;
@group(0) @binding(24) var sky_view_lut: texture_2d<f32>;
@group(0) @binding(25) var aerial_perspective_lut: texture_3d<f32>;
@group(0) @binding(26) var<storage, read> clouds: array<VolumetricCloud>;
@group(0) @binding(27) var blue_noise_tex: texture_2d<f32>;
@group(0) @binding(28) var blue_noise_sampler: sampler;
@group(0) @binding(29) var transmittance_lut: texture_2d<f32>;
@group(0) @binding(30) var cloud_shape_noise_tex: texture_3d<f32>;
@group(0) @binding(31) var cloud_detail_noise_tex: texture_3d<f32>;
@group(0) @binding(32) var cloud_weather_tex: texture_2d<f32>;
@group(0) @binding(33) var cloud_radiance_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(34) var cloud_radiance_history_tex: texture_2d<f32>;
@group(0) @binding(35) var cloud_transmittance_tex: texture_storage_2d<r16float, write>;
@group(0) @binding(36) var cloud_transmittance_history_tex: texture_2d<f32>;
@group(0) @binding(37) var cloud_shadow_optical_depth_tex: texture_storage_2d<r16float, write>;
@group(0) @binding(38) var cloud_directional_shadow_tex: texture_2d<f32>;

// GI
struct GiParams { quality: u32, debug_mode: u32, mode: u32, _pad: u32, };
@group(0) @binding(11) var<uniform> gi_params: GiParams;
@group(0) @binding(12) var gi_sdf:      texture_3d<f32>;
@group(0) @binding(13) var gi_sampler:  sampler;
@group(0) @binding(14) var gi_history:  texture_2d<f32>;
@group(0) @binding(15) var gi_noisy:    texture_storage_2d<rgba16float, write>;
@group(0) @binding(16) var gi_radiance: texture_3d<f32>;
@group(0) @binding(18) var lightmap:    texture_2d<f32>;

struct LightListHeader { count: u32, };
@group(0) @binding(19) var<uniform> light_header: LightListHeader;
@group(0) @binding(20) var<storage, read> light_indices: array<u32>;


fn sample_blue_noise(pixel: vec2<u32>, frame_number: i32) -> f32 {
    let dims = vec2<u32>(textureDimensions(blue_noise_tex, 0));
    let frame = u32(max(frame_number, 0));
    let offset = vec2<u32>((frame * 5u + frame / 3u) % dims.x, (frame * 7u + frame / 5u) % dims.y);
    let coord = (pixel + offset) % dims;
    return textureSampleLevel(blue_noise_tex, blue_noise_sampler, (vec2<f32>(coord) + vec2<f32>(0.5)) / vec2<f32>(dims), 0.0).r;
}

fn hash31(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
    let d = dot(q, q.yzx + vec3<f32>(33.33));
    return fract((q.x + q.y) * (q.z + d));
}

fn cloud_density(cloud: VolumetricCloud, p: vec3<f32>) -> f32 {
    let center = cloud.center_base_thickness.xyz;
    let base_radius = max(cloud.center_base_thickness.w, 0.001);
    let thickness = max(cloud.shape_params.x, 0.001);
    let rel = p - center;
    let altitude = length(rel) - base_radius;
    let height01 = clamp(altitude / thickness, 0.0, 1.0);

    let up = normalize(select(vec3<f32>(0.0, 1.0, 0.0), rel, length(rel) > 1e-4));
    let east_seed = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(up.y) > 0.95);
    let tangent_x = normalize(cross(east_seed, up));
    let tangent_z = cross(up, tangent_x);
    let wind = (tangent_x * cloud.wind_steps.x + tangent_z * cloud.wind_steps.y) * cloud.wind_steps.z * params.current_time;

    let weather_uv = (vec2<f32>(dot(p - center, tangent_x), dot(p - center, tangent_z)) + cloud.weather_params.yz + vec2<f32>(params.current_time * cloud.wind_steps.z * 0.02)) * max(cloud.weather_params.x, 0.0001);
    let weather = textureSampleLevel(cloud_weather_tex, tex_sampler, weather_uv, 0.0);
    let weather_coverage = clamp(weather.r + (cloud.coverage_density_noise_phase.x - 0.5) * 1.4, 0.0, 1.0);
    let weather_density = clamp(weather.g, 0.0, 1.0);
    let weather_type = clamp(mix(cloud.detail_params.y, weather.b, clamp(cloud.weather_params.w, 0.0, 1.0)), 0.0, 1.0);

    // Curl-like domain distortion from detail noise keeps wind motion from looking like a sliding sheet.
    let curl_strength = clamp(cloud.detail_params.w, 0.0, 2.0);
    let curl_a = textureSampleLevel(cloud_detail_noise_tex, tex_sampler, (p + wind) * 0.021, 0.0).rgb - vec3<f32>(0.5);
    let distorted_p = p + wind + (curl_a.zxy - curl_a.yxz) * (curl_strength * thickness * 0.08);

    let shape_scale = max(cloud.shape_params.z, max(cloud.coverage_density_noise_phase.z, 0.001));
    let detail_scale = max(cloud.shape_params.w, shape_scale * 4.0);
    let shape = textureSampleLevel(cloud_shape_noise_tex, tex_sampler, distorted_p * shape_scale, 0.0);
    let detail = textureSampleLevel(cloud_detail_noise_tex, tex_sampler, distorted_p * detail_scale, 0.0);

    let low_frequency = dot(shape.rgb, vec3<f32>(0.625, 0.25, 0.125));
    let high_frequency = dot(detail.rgb, vec3<f32>(0.5, 0.35, 0.15));

    let base_fade = smoothstep(0.0, mix(0.05, 0.22, weather_type), height01);
    let anvil = clamp(cloud.detail_params.z, 0.0, 1.0);
    let top_start = mix(0.55, 0.86, weather_type) - anvil * 0.18;
    let top_fade = 1.0 - smoothstep(top_start, 1.0, height01);
    let anvil_lift = mix(1.0, smoothstep(0.35, 0.9, height01), anvil * weather_type);
    let height_gradient = base_fade * top_fade * anvil_lift;

    let macro_variation = mix(1.0, mix(0.65, 1.35, weather.a), clamp(cloud.weather_params.w, 0.0, 1.0));
    let coverage_threshold = mix(1.0, 0.08, weather_coverage);
    let billow = smoothstep(coverage_threshold - 0.18, coverage_threshold + 0.18, low_frequency * macro_variation);
    let erosion = clamp(cloud.detail_params.x, 0.0, 1.0) * smoothstep(0.18, 1.0, height01);
    let eroded = max(billow - high_frequency * erosion, 0.0);

    return eroded * height_gradient * weather_density * max(cloud.coverage_density_noise_phase.y, 0.0);
}
fn cloud_shell_interval(cloud: VolumetricCloud, ro: vec3<f32>, rd: vec3<f32>, scene_depth: f32) -> vec2<f32> {
    let center = cloud.center_base_thickness.xyz;
    let inner_radius = max(cloud.center_base_thickness.w, 0.001);
    let outer_radius = inner_radius + max(cloud.shape_params.x, 0.001);
    let outer = ray_sphere_forward_interval(ro, rd, center, outer_radius);
    if (outer.y <= 0.0 || outer.x > outer.y) {
        return vec2<f32>(1e20, -1e20);
    }

    let dist_from_center = length(ro - center);
    var t0 = max(outer.x, 0.0);
    var t1 = min(outer.y, scene_depth);
    let inner = ray_sphere_forward_interval(ro, rd, center, inner_radius);

    if (dist_from_center < inner_radius && inner.y > 0.0) {
        t0 = max(t0, inner.y);
    } else if (inner.x > t0 && inner.x < t1) {
        // Stop at the planet-facing side of the hollow shell. The far side is
        // hidden by the planet for normal surface/space camera views.
        t1 = inner.x;
    }

    return vec2<f32>(t0, t1);
}

fn cloud_shell_exit_distance(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    let center = cloud.center_base_thickness.xyz;
    let outer_radius = max(cloud.center_base_thickness.w, 0.001) + max(cloud.shape_params.x, 0.001);
    let hit = ray_sphere_forward_interval(p, light_dir, center, outer_radius);
    return max(hit.y, 0.0);
}

fn cloud_henyey_greenstein(g: f32, mu: f32) -> f32 {
    let gg = g * g;
    return (1.0 - gg) / max(0.001, pow(1.0 + gg - 2.0 * g * mu, 1.5));
}

fn cloud_phase_lobe(cloud: VolumetricCloud, mu: f32) -> f32 {
    let forward_g = clamp(cloud.lighting_params0.x, 0.0, 0.95);
    let backward_g = -clamp(abs(cloud.lighting_params0.y), 0.0, 0.95);
    let forward_blend = clamp(cloud.lighting_params0.z, 0.0, 1.0);
    let forward_lobe = cloud_henyey_greenstein(forward_g, mu);
    let backward_lobe = cloud_henyey_greenstein(backward_g, mu);
    return mix(backward_lobe, forward_lobe, forward_blend);
}

fn cloud_powder_term(cloud: VolumetricCloud, sigma: f32, light_t: f32) -> f32 {
    let strength = clamp(cloud.lighting_params0.w, 0.0, 2.0);
    let density_powder = 1.0 - exp(-max(sigma, 0.0) * 2.25);
    let edge_visibility = smoothstep(0.05, 0.95, light_t);
    return 1.0 + strength * density_powder * edge_visibility;
}

fn cloud_silver_lining(cloud: VolumetricCloud, mu: f32, light_t: f32) -> f32 {
    let strength = clamp(cloud.lighting_params1.x, 0.0, 4.0);
    let rim = pow(clamp(mu * 0.5 + 0.5, 0.0, 1.0), 16.0);
    return 1.0 + strength * rim * smoothstep(0.08, 0.75, light_t);
}

fn cloud_multi_scatter_lighting(cloud: VolumetricCloud, sigma: f32, mu: f32, light_t: f32, sun_col: vec3<f32>) -> vec3<f32> {
    let strength = clamp(cloud.multi_scatter.x, 0.0, 2.0);
    if (strength <= 0.0) { return vec3<f32>(0.0); }

    let octaves = min(max(u32(cloud.multi_scatter.y), 0u), 6u);
    let attenuation = clamp(cloud.multi_scatter.z, 0.0, 1.0);
    let eccentricity = clamp(cloud.multi_scatter.w, 0.0, 1.0);
    let density_lift = 1.0 - exp(-max(sigma, 0.0) * 0.75);
    let self_shadow_lift = 1.0 - light_t;
    let ambient_visibility = mix(0.35, 1.0, light_t);

    var energy = 0.0;
    var phase_scale = attenuation;
    var g_scale = eccentricity;
    for (var octave: u32 = 0u; octave < octaves; octave = octave + 1u) {
        let octave_g = clamp(cloud.coverage_density_noise_phase.w * g_scale, -0.95, 0.95);
        energy += phase_scale * cloud_henyey_greenstein(octave_g, mu);
        phase_scale *= attenuation;
        g_scale *= eccentricity;
    }

    let ambient_energy = density_lift * (0.25 + 0.75 * self_shadow_lift) * ambient_visibility;
    return sun_col * clamp(strength * (energy * 0.045 * ambient_visibility + ambient_energy * 0.18), 0.0, 0.75);
}

fn cloud_blue_noise_jitter(pixel: vec2<u32>, frame: u32, sample_index: u32) -> f32 {
    let tile = vec2<f32>(pixel & vec2<u32>(127u));
    return hash31(vec3<f32>(tile * vec2<f32>(0.75487766, 0.56984029), f32((frame + sample_index * 37u) & 255u) * 0.61803399));
}

fn ray_sphere_forward_interval(ro: vec3<f32>, rd: vec3<f32>, center: vec3<f32>, radius: f32) -> vec2<f32> {
    let oc = ro - center;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - radius * radius;
    let h = b * b - c;
    if (h < 0.0) {
        return vec2<f32>(1e20, -1e20);
    }
    let sqrt_h = sqrt(h);
    return vec2<f32>(-b - sqrt_h, -b + sqrt_h);
}

fn cloud_planet_shadow_for_atmosphere(atmo: Atmosphere, cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, cloud_exit_dist: f32) -> f32 {
    let planet_center = atmo.center_radius.xyz;
    let planet_radius = max(atmo.center_radius.w, 0.0);
    let atmosphere_radius = max(atmo.atmo_g_height.x, planet_radius);
    if (planet_radius <= 0.0 || atmosphere_radius <= 0.0) {
        return 1.0;
    }

    let atmosphere_hit = ray_sphere_forward_interval(p, light_dir, planet_center, atmosphere_radius);
    let atmosphere_exit_dist = select(atmosphere_hit.y, 1e20, atmosphere_hit.y <= 0.0);
    let max_shadow_dist = min(cloud_exit_dist, atmosphere_exit_dist);
    if (max_shadow_dist <= 0.0) {
        return 1.0;
    }

    let to_sample = p - planet_center;
    let dist_to_axis = length(cross(to_sample, light_dir));
    let penumbra = max(cloud.light_padding.z, 0.0);
    let soft_radius = max(penumbra, 1e-3);
    let soft_shadow = smoothstep(planet_radius - soft_radius, planet_radius + soft_radius, dist_to_axis);

    let planet_hit = ray_sphere_forward_interval(p, light_dir, planet_center, planet_radius);
    let hits_planet_before_exit = planet_hit.y >= 0.0 && max(planet_hit.x, 0.0) <= max_shadow_dist;
    return select(1.0, soft_shadow, hits_planet_before_exit);
}

fn cloud_planet_shadow(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, cloud_exit_dist: f32) -> f32 {
    var shadow = 1.0;
    for (var ai: u32 = 0u; ai < params.atmo_count && ai < MAX_ATMOSPHERES; ai = ai + 1u) {
        shadow = min(shadow, cloud_planet_shadow_for_atmosphere(params.atmos[ai], cloud, p, light_dir, cloud_exit_dist));
    }
    return shadow;
}

fn cloud_object_shadow_transmittance(p: vec3<f32>, light_dir: vec3<f32>, max_dist: f32) -> f32 {
    if (max_dist <= 0.0 || params.total_bvh_nodes == 0u) {
        return 1.0;
    }

    // Reuse the directional-light visibility path for object-to-cloud shadows:
    // trace a single hard visibility ray from the cloud sample toward the sun.
    // The caller decides how many cloud raymarch samples may pay this TLAS cost.
    let shadow_dist = min(max_dist, 1000.0);
    let origin = p + light_dir * 0.02;
    return select(0.0, 1.0, is_visible(origin, origin + light_dir * shadow_dist, 0xffffffffu, 0xffffffffu));
}

fn cloud_object_shadow_sample_enabled(cloud: VolumetricCloud, sample_index: u32, primary_steps: u32) -> bool {
    if (params.cloud_object_shadows_enabled == 0u || params.raytraced_shadows_enabled == 0u || params.shadow_quality == SHADOW_QUALITY_OFF) { return false; }
    let quality = min(u32(max(cloud.light_padding.w, 0.0)), 4u);
    if (quality == 0u) {
        return false;
    }

    // Quality is a feature/cost knob stored per cloud. It controls a small
    // sparse subset of primary cloud samples that run object visibility rays.
    let max_checked_samples = min(primary_steps, quality * 2u);
    let stride = max(1u, primary_steps / max_checked_samples);
    return (sample_index % stride) == 0u;
}

fn cloud_light_transmittance(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, jitter: f32, object_shadow: f32) -> f32 {
    let steps = max(1u, min(u32(cloud.light_padding.x), 32u));
    let max_dist = cloud_shell_exit_distance(cloud, p, light_dir);
    let planet_shadow = cloud_planet_shadow(cloud, p, light_dir, max_dist);
    if (planet_shadow <= 0.0) {
        return 0.0;
    }
    let dt = max_dist / f32(steps);
    var optical_depth = 0.0;
    for (var li: u32 = 0u; li < steps; li = li + 1u) {
        let lp = p + light_dir * ((f32(li) + jitter) * dt);
        optical_depth += cloud_density(cloud, lp) * dt;
    }
    let self_shadow = exp(-optical_depth * max(cloud.light_padding.y, 0.0));
    return self_shadow * planet_shadow * clamp(object_shadow, 0.0, 1.0);
}

fn cloud_shadow_transmittance_for_volume(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, jitter: f32) -> f32 {
    let shadow_strength = max(cloud.light_padding.y, 0.0);
    if (shadow_strength <= 0.0) {
        return 1.0;
    }

    let interval = cloud_shell_interval(cloud, p, light_dir, 1e9);
    let t0 = interval.x;
    let t1 = interval.y;
    if (t1 <= t0) {
        return 1.0;
    }

    // Surface cloud shadows use a deliberately lower-resolution march than
    // volumetric cloud lighting. This keeps the per-hit direct-light path cheap
    // while still matching the same density field and wind animation.
    let requested_steps = max(cloud.light_padding.x * 0.5, 1.0);
    let steps = max(1u, min(u32(requested_steps), 12u));
    let dt = (t1 - t0) / f32(steps);
    var optical_depth = 0.0;
    for (var si: u32 = 0u; si < steps; si = si + 1u) {
        let sp = p + light_dir * (t0 + (f32(si) + jitter) * dt);
        optical_depth += cloud_density(cloud, sp) * dt;
    }
    return exp(-optical_depth * shadow_strength);
}


const CLOUD_SHADOW_WORLD_EXTENT: f32 = 4096.0;
const CLOUD_SHADOW_RAY_LENGTH: f32 = 65536.0;
const CLOUD_SHADOW_MARCH_STEPS: u32 = 64u;

fn sun_shadow_basis(light_dir: vec3<f32>) -> mat3x3<f32> {
    let sun_dir = normalize(light_dir);
    let up_hint = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(sun_dir.y) > 0.95);
    let sx = normalize(cross(up_hint, sun_dir));
    let sy = normalize(cross(sun_dir, sx));
    return mat3x3<f32>(sx, sy, sun_dir);
}

fn world_to_sun_shadow_uv(p: vec3<f32>, basis: mat3x3<f32>) -> vec2<f32> {
    let sun_xy = vec2<f32>(dot(p, basis[0]), dot(p, basis[1]));
    return sun_xy / CLOUD_SHADOW_WORLD_EXTENT + vec2<f32>(0.5);
}

fn sun_shadow_texel_world_origin(uv: vec2<f32>, basis: mat3x3<f32>) -> vec3<f32> {
    let sun_xy = (uv - vec2<f32>(0.5)) * CLOUD_SHADOW_WORLD_EXTENT;
    return basis[0] * sun_xy.x + basis[1] * sun_xy.y - basis[2] * (CLOUD_SHADOW_RAY_LENGTH * 0.5);
}

fn directional_cloud_shadow_optical_depth(uv: vec2<f32>, light_dir: vec3<f32>) -> f32 {
    let basis = sun_shadow_basis(light_dir);
    let ray_origin = sun_shadow_texel_world_origin(uv, basis);
    let dt = CLOUD_SHADOW_RAY_LENGTH / f32(CLOUD_SHADOW_MARCH_STEPS);
    var optical_depth = 0.0;
    for (var si: u32 = 0u; si < CLOUD_SHADOW_MARCH_STEPS; si = si + 1u) {
        let p = ray_origin + basis[2] * ((f32(si) + 0.5) * dt);
        for (var ci: u32 = 0u; ci < params.cloud_count && ci < MAX_VOLUMETRIC_CLOUDS; ci = ci + 1u) {
            let cloud = clouds[ci];
            optical_depth += cloud_density(cloud, p) * dt * max(cloud.light_padding.y, 0.0);
        }
    }
    return optical_depth;
}

@compute @workgroup_size(8,8)
fn cloud_shadow_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(cloud_shadow_optical_depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let uv = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let light_dir = normalize(-params.dir_light_dir.xyz);
    let od = directional_cloud_shadow_optical_depth(uv, light_dir);
    textureStore(cloud_shadow_optical_depth_tex, vec2<i32>(id.xy), vec4<f32>(od, 0.0, 0.0, 1.0));
}

fn cached_cloud_shadow_transmittance(p: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    // Directional-cache approximation: project world position onto a stable plane
    // perpendicular to the sun and reuse the precomputed optical-depth atlas.
    let basis = sun_shadow_basis(light_dir);
    let uv = world_to_sun_shadow_uv(p, basis);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return 1.0;
    }
    let od = textureSampleLevel(cloud_directional_shadow_tex, tex_sampler, uv, 0.0).r;
    return exp(-od);
}

fn cloud_shadow_transmittance(p: vec3<f32>, light_dir: vec3<f32>, jitter: f32) -> f32 {
    if (params.cloud_shadow_mode == 0u) { return cached_cloud_shadow_transmittance(p, light_dir); }
    var transmittance = 1.0;
    for (var ci: u32 = 0u; ci < params.cloud_count && ci < MAX_VOLUMETRIC_CLOUDS; ci = ci + 1u) {
        transmittance *= cloud_shadow_transmittance_for_volume(clouds[ci], p, light_dir, fract(jitter + f32(ci) * 0.61803399));
        if (transmittance < T_EARLY_OUT) {
            return 0.0;
        }
    }
    return transmittance;
}

fn transmittance_lut_uv(atmo: Atmosphere, origin: vec3<f32>, light_dir: vec3<f32>) -> vec2<f32> {
    let up = normalize(origin - atmo.center_radius.xyz);
    let atmosphere_thickness = max(atmo.atmo_g_height.x - atmo.center_radius.w, 1e-3);
    let altitude = clamp(length(origin - atmo.center_radius.xyz) - atmo.center_radius.w, 0.0, atmosphere_thickness);
    let altitude_u = altitude / atmosphere_thickness;
    let zenith_u = dot(normalize(light_dir), up) * 0.5 + 0.5;
    return clamp(vec2<f32>(zenith_u, altitude_u), vec2<f32>(0.0), vec2<f32>(1.0));
}

fn sample_transmittance_lut(atmo: Atmosphere, origin: vec3<f32>, light_dir: vec3<f32>) -> vec3<f32> {
    return textureSampleLevel(transmittance_lut, tex_sampler, transmittance_lut_uv(atmo, origin, light_dir), 0.0).xyz;
}

fn atmosphere_sun_transmittance(p: vec3<f32>, sun_dir: vec3<f32>) -> vec3<f32> {
    var trans = vec3<f32>(1.0);
    for (var ai: u32 = 0u; ai < params.atmo_count && ai < MAX_ATMOSPHERES; ai = ai + 1u) {
        let atmo = params.atmos[ai];
        let atmosphere_radius = max(atmo.atmo_g_height.x, atmo.center_radius.w);
        let rel = p - atmo.center_radius.xyz;
        if (length(rel) <= atmosphere_radius) {
            var atmo_trans = sample_transmittance_lut(atmo, p, sun_dir);
            let planet_hit = ray_sphere_forward_interval(p - atmo.center_radius.xyz, sun_dir, vec3<f32>(0.0), atmo.center_radius.w);
            if (planet_hit.y > 0.0 && planet_hit.x > 0.0) {
                atmo_trans = vec3<f32>(0.0);
            }
            trans *= atmo_trans;
        }
    }
    return clamp(trans, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn sky_ambient_probe(up: vec3<f32>, sun_dir: vec3<f32>) -> vec3<f32> {
    let safe_up = normalize(select(vec3<f32>(0.0, 1.0, 0.0), up, length(up) > 1e-4));
    let sun_tangent = sun_dir - safe_up * dot(sun_dir, safe_up);
    let safe_sun_tangent = normalize(select(vec3<f32>(1.0, 0.0, 0.0), sun_tangent, length(sun_tangent) > 1e-4));
    let horizon_dir = normalize(safe_up * 0.18 + safe_sun_tangent * 0.82);
    let zenith_sample = sample_sky_view_lut(safe_up);
    let horizon_sample = sample_sky_view_lut(horizon_dir);
    let anti_sun_sample = sample_sky_view_lut(normalize(safe_up * 0.45 - sun_dir * 0.55));
    let sun_height = clamp(dot(sun_dir, safe_up) * 0.5 + 0.5, 0.0, 1.0);
    let sunset_warmth = smoothstep(0.02, 0.45, 1.0 - abs(dot(sun_dir, safe_up)));
    let base_probe = zenith_sample * 0.55 + horizon_sample * 0.30 + anti_sun_sample * 0.15;
    let warm_tint = vec3<f32>(1.22, 0.72, 0.42);
    let cool_shadow_tint = vec3<f32>(0.50, 0.62, 0.82);
    let tint = mix(cool_shadow_tint, warm_tint, sunset_warmth * (1.0 - sun_height * 0.35));
    return max(base_probe * tint * 0.18, params.skycolor.xyz * 0.015);
}

struct CloudCompositeResult {
    color: vec3<f32>,
    radiance: vec3<f32>,
    transmittance: f32,
};

fn cloud_history_uv(rd: vec3<f32>, scene_depth: f32) -> vec2<f32> {
    let world = params.camera_pos.xyz + rd * min(scene_depth, 10000.0);
    let prev = params.prev_view_proj * vec4<f32>(world, 1.0);
    if (abs(prev.w) < 1e-5) { return vec2<f32>(-1.0); }
    let ndc = prev.xy / prev.w;
    return ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
}

fn cloud_temporal_rejection(pixel: vec2<u32>, uv: vec2<f32>, scene_depth: f32, transmittance: f32) -> f32 {
    if (params.cloud_temporal_quality == 0u || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) { return 0.0; }
    let dims = vec2<f32>(textureDimensions(cloud_radiance_history_tex, 0));
    let motion = length((uv - (vec2<f32>(pixel) + vec2<f32>(0.5)) / dims) * dims);
    let motion_reject = smoothstep(48.0, 6.0, motion);
    let depth_reject = select(0.35, 1.0, scene_depth < 1e8);
    let prev_t = textureSampleLevel(cloud_transmittance_history_tex, tex_sampler, uv, 0.0).r;
    let trans_reject = smoothstep(0.45, 0.08, abs(prev_t - transmittance));
    return clamp(params.cloud_history_weight, 0.0, 0.98) * motion_reject * depth_reject * trans_reject;
}

fn composite_clouds(ro: vec3<f32>, rd: vec3<f32>, scene_depth: f32, base_color: vec3<f32>, pixel: vec2<u32>) -> CloudCompositeResult {
    var radiance = vec3<f32>(0.0);
    var transmittance = 1.0;
    let sun_dir = normalize(-params.dir_light_dir.xyz);
    let sun_col = params.dir_light_color.xyz * max(params.dir_light_dir.w, 0.0);
    let camera_up = normalize(select(vec3<f32>(0.0, 1.0, 0.0), ro, length(ro) > 1e-4));
    let camera_sky_ambient = sky_ambient_probe(camera_up, sun_dir);
    for (var ci: u32 = 0u; ci < params.cloud_count && ci < MAX_VOLUMETRIC_CLOUDS; ci = ci + 1u) {
        let cloud = clouds[ci];
        let interval = cloud_shell_interval(cloud, ro, rd, scene_depth);
        let t0 = interval.x;
        let t1 = interval.y;
        if (t1 <= t0) { continue; }
        let requested_steps = select(u32(cloud.shape_params.y), params.cloud_sample_count, params.cloud_sample_count > 0u);
        let steps = max(1u, min(requested_steps, 96u));
        let dt = (t1 - t0) / f32(steps);
        let primary_jitter = cloud_blue_noise_jitter(pixel, u32(params.frame_number), ci * 131u);
        for (var sidx: u32 = 0u; sidx < steps; sidx = sidx + 1u) {
            let t = t0 + (f32(sidx) + primary_jitter) * dt;
            let p = ro + rd * t;
            let sigma = cloud_density(cloud, p);
            if (sigma <= 0.0) { continue; }
            let mu = clamp(dot(rd, sun_dir), -1.0, 1.0);
            let phase = cloud_phase_lobe(cloud, mu);
            let jitter = cloud_blue_noise_jitter(pixel, u32(params.frame_number), sidx + ci * 97u);
            var object_shadow = 1.0;
            if (cloud_object_shadow_sample_enabled(cloud, sidx, steps)) {
                object_shadow = cloud_object_shadow_transmittance(p, sun_dir, cloud_shell_exit_distance(cloud, p, sun_dir));
            }
            let light_t = cloud_light_transmittance(cloud, p, sun_dir, jitter, object_shadow);
            let local_up = normalize(select(camera_up, p - cloud.center_base_thickness.xyz, length(p - cloud.center_base_thickness.xyz) > 1e-4));
            let sun_atmosphere_t = atmosphere_sun_transmittance(p, sun_dir);
            let atmosphere_sun_col = sun_col * sun_atmosphere_t;
            let sky_ambient = max(camera_sky_ambient, sky_ambient_probe(local_up, sun_dir));
            let multi_scatter = cloud_multi_scatter_lighting(cloud, sigma, mu, light_t, atmosphere_sun_col + sky_ambient * 0.55);
            let absorb = exp(-sigma * dt);
            let scatter = (1.0 - absorb) * transmittance;
            let powder = cloud_powder_term(cloud, sigma, light_t);
            let silver = cloud_silver_lining(cloud, mu, light_t);
            let direct_lighting = atmosphere_sun_col * phase * 0.08 * light_t * powder * silver;
            let ambient_lighting = sky_ambient * mix(0.95, 0.30, light_t) * mix(0.85, 1.15, powder - 1.0);
            let cloud_lighting = direct_lighting + multi_scatter + ambient_lighting;
            let sample_lighting = min(cloud_lighting, atmosphere_sun_col * (1.35 + 0.45 * cloud.lighting_params1.x) + sky_ambient * 2.0 + vec3<f32>(0.015));
            let aerial_sample = textureSampleLevel(aerial_perspective_lut, tex_sampler, vec3<f32>(atmosphere_lut_uv(rd), clamp(sqrt(clamp((t - 2.0) / 998.0, 0.0, 1.0)), 0.0, 1.0)), 0.0);
            let camera_tinted_lighting = aerial_sample.xyz + sample_lighting * vec3<f32>(aerial_sample.w);
            radiance += scatter * camera_tinted_lighting;
            transmittance *= absorb;
            if (transmittance < 0.01) { break; }
        }
    }
    let history_uv = cloud_history_uv(rd, scene_depth);
    let history_w = cloud_temporal_rejection(pixel, history_uv, scene_depth, transmittance);
    var resolved_radiance = radiance;
    var resolved_transmittance = transmittance;
    if (history_w > 0.0) {
        let history_radiance = textureSampleLevel(cloud_radiance_history_tex, tex_sampler, history_uv, 0.0).xyz;
        let history_transmittance = textureSampleLevel(cloud_transmittance_history_tex, tex_sampler, history_uv, 0.0).r;
        resolved_radiance = mix(radiance, history_radiance, history_w);
        resolved_transmittance = mix(transmittance, history_transmittance, history_w);
    }
    let integrated = resolved_radiance + base_color * resolved_transmittance;
    return CloudCompositeResult(sample_aerial_perspective_lut(rd, scene_depth, integrated), resolved_radiance, resolved_transmittance);
}

// -----------------------------
// RNG / Math helpers
// -----------------------------
const PI:  f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;
const NO_HIT: f32 = 1e20;

fn step_rng(state: u32) -> u32 { return state * 747796405u + 1u; }

fn rand(state: ptr<function, u32>) -> f32 {
    *(state) = step_rng(*(state));
    var word: u32 = ((*(state) >> ((*(state) >> 28u) + 4u)) ^ *(state)) * 277803737u;
    word = (word >> 22u) ^ word;
    return f32(word) / 4294967295.0;
}
// Physically correct refraction for a ray direction `rd` (propagating away from the eye).
// `n` is the *outward* geometric normal of the surface (not the microfacet half vector).
// `ior` is the material index of refraction (> 1). Handles both enter/exit.
fn refract_ray(rd_in: vec3<f32>, n_in: vec3<f32>, ior: f32) -> vec3<f32> {
    var rd = normalize(rd_in);
    var n  = normalize(n_in);

    // Assume outside air = 1.0
    var eta = 1.0 / max(ior, 1e-4);  // default: air -> material
    // If we are *inside* (ray pointing in the same hemi as the outward normal),
    // flip the normal and swap media: material -> air
    if (dot(rd, n) > 0.0) {
        n   = -n;
        eta = ior;                   // material -> air
    }

    let cosi = -dot(rd, n);          // > 0
    let k    = 1.0 - eta*eta * (1.0 - cosi*cosi);
    if (k <= 0.0) {                  // total internal reflection
        return vec3<f32>(0.0);
    }
    return normalize(eta * rd + (eta * cosi - sqrt(k)) * n);
}

fn init_rng(pixel: vec2<u32>, frame: u32) -> u32 {
    var seed = pixel.x * 1973u + pixel.y * 9277u + frame * 26699u;
    seed = seed ^ (seed << 13u);
    seed = seed ^ (seed >> 17u);
    seed = seed ^ (seed << 5u);
    return seed;
}

fn random_in_unit_sphere(state: ptr<function, u32>) -> vec3<f32> {
    return normalize(vec3<f32>(rand(state) * 2.0 - 1.0, rand(state) * 2.0 - 1.0, rand(state) * 2.0 - 1.0));
}

fn random_in_unit_cube(state: ptr<function, u32>) -> vec3<f32> {
    return vec3<f32>(rand(state) * 2.0 - 1.0, rand(state) * 2.0 - 1.0, rand(state) * 2.0 - 1.0);
}

fn random_cube_surface(state: ptr<function, u32>) -> vec3<f32> {
    var p = random_in_unit_cube(state);
    let a = abs(p);
    if (a.x >= a.y && a.x >= a.z) { p.x = sign(p.x); }
    else if (a.y >= a.x && a.y >= a.z) { p.y = sign(p.y); }
    else { p.z = sign(p.z); }
    return p;
}

fn random_in_unit_disk(state: ptr<function, u32>) -> vec2<f32> {
    let r = sqrt(rand(state));
    let phi = TAU * rand(state);
    return vec2<f32>(r * cos(phi), r * sin(phi));
}

// ---- Atmospheric scattering helpers ----

fn atmosphere_lut_uv(dir: vec3<f32>) -> vec2<f32> {
    let d = normalize(dir);
    let u = atan2(d.x, d.z) / TAU + 0.5;
    let v = clamp(d.y * 0.5 + 0.5, 0.0, 1.0);
    return vec2<f32>(u, v);
}

fn sample_sky_view_lut(dir: vec3<f32>) -> vec3<f32> {
    return textureSampleLevel(sky_view_lut, tex_sampler, atmosphere_lut_uv(dir), 0.0).xyz;
}

fn sample_aerial_perspective_lut(dir: vec3<f32>, max_t: f32, background: vec3<f32>) -> vec3<f32> {
    let uv = atmosphere_lut_uv(dir);
    let z = clamp(sqrt(clamp((max_t - 2.0) / 998.0, 0.0, 1.0)), 0.0, 1.0);
    let sample = textureSampleLevel(aerial_perspective_lut, tex_sampler, vec3<f32>(uv, z), 0.0);
    return sample.xyz + background * vec3<f32>(sample.w);
}

fn ray_sphere_intersect(start: vec3<f32>, dir: vec3<f32>, radius: f32) -> vec2<f32> {
    let a = dot(dir, dir);
    let b = 2.0 * dot(dir, start);
    let c = dot(start, start) - radius * radius;
    let d = b * b - 4.0 * a * c;
    if (d < 0.0) { return vec2<f32>(1e9, -1e9); }
    let sqrt_d = sqrt(d);
    return vec2<f32>((-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a));
}


fn absorption_layer_density(layer: vec4<f32>, altitude: f32) -> f32 {
    let exp_term = layer.x * exp(clamp(layer.y * altitude, -80.0, 80.0));
    let linear_term = layer.z * altitude + layer.w;
    return max(exp_term + linear_term, 0.0);
}

fn absorption_profile_density(atmo: Atmosphere, height: f32) -> f32 {
    let lower_width = max(atmo.absorb_params.x, 0.0);
    let upper_width = max(atmo.absorb_params.y, 0.0);
    let density_scale = max(atmo.absorb_params.z, 0.0);
    let lower_density = select(0.0, absorption_layer_density(atmo.ozone_params, height), height <= lower_width);
    let upper_altitude = max(height - lower_width, 0.0);
    let upper_density = select(0.0, absorption_layer_density(atmo.absorption_layer_params, height), height > lower_width && upper_altitude <= upper_width);
    return density_scale * clamp(max(lower_density, upper_density), 0.0, 1.0);
}

fn estimate_multi_scattering(
    atmo: Atmosphere,
    tauR: f32,
    tauM: f32,
    tauA: f32,
    dir: vec3<f32>,
    light_dir: vec3<f32>,
    light_intensity: vec3<f32>
) -> vec3<f32> {
    // Low-cost higher-order scattering approximation.  It treats light that was
    // already scattered once as a broad, mostly isotropic source whose energy is
    // limited by single-scattering albedo and view optical depth.
    let strength = max(atmo.multi_scatter_params.x, 0.0);
    if (strength <= 0.0) { return vec3<f32>(0.0); }

    let falloff = max(atmo.multi_scatter_params.y, 1e-3);
    let phase_boost = atmo.multi_scatter_params.z;
    let ambient_mix = clamp(atmo.multi_scatter_params.w, 0.0, 1.0);

    let scatter_depth = atmo.ray_beta.xyz * vec3<f32>(tauR) + atmo.mie_beta.xyz * vec3<f32>(tauM);
    let extinction_depth = scatter_depth + atmo.absorption_beta.xyz * vec3<f32>(tauA);
    let albedo = scatter_depth / max(extinction_depth, vec3<f32>(1e-5));

    let view_energy = vec3<f32>(1.0) - exp(-extinction_depth * falloff);
    let mu = clamp(dot(dir, light_dir), -1.0, 1.0);
    let forward_fill = mix(1.0, pow(max(0.0, mu) + 0.25, 2.0), clamp(phase_boost, 0.0, 1.0));
    let ambient_source = mix(light_intensity, atmo.ambient_beta.xyz, ambient_mix);

    return strength * ambient_source * albedo * view_energy * forward_fill;
}

fn calculate_scattering(
    atmo: Atmosphere,
    start: vec3<f32>,          // camera-relative
    dir: vec3<f32>,            // normalized
    max_t: f32,
    light_dir: vec3<f32>,      // normalized
    light_intensity: vec3<f32>,
    pixel: vec2<u32>
) -> Scattering {
    // Everything operates in camera-relative space.
    let center = atmo.center_radius.xyz;

    // Intersect view ray with atmosphere shell (relative to center).
    var pos_rel = start - center;
    let seg = ray_sphere_intersect(pos_rel, dir, atmo.atmo_g_height.x);
    var t0 = max(seg.x, 0.0);
    var t1 = min(seg.y, max_t);
    if (t0 > t1) { return Scattering(vec3<f32>(0.0), vec3<f32>(1.0)); }

    // Phase (cheap HG)
    let mu   = dot(dir, light_dir);
    let mumu = mu * mu;
    let g    = atmo.atmo_g_height.y;
    let gg   = g * g;
    let phase_ray = 0.05968310366 * (1.0 + mumu);                    // 3/(16π)
    let den = 1.0 + gg - 2.0 * mu * g;
    let phase_mie = 0.11936620732 * (1.0 - gg) * (1.0 + mumu) / (den * sqrt(den)); // 3/(8π)

    // Precompute reciprocals
    let invHR = 1.0 / atmo.atmo_g_height.z;
    let invHM = 1.0 / atmo.atmo_g_height.w;

    // Adaptive steps (cap by authoring)
    let len = t1 - t0;
    let base_steps = min(i32(atmo.absorb_params.w), MAX_SCATTER_STEPS);
    let target_dl  = max(0.5 * atmo.atmo_g_height.z, 1e-3);
    let steps_f = clamp(ceil(len / target_dl), 4.0, f32(base_steps));
    let steps = i32(steps_f);

    let dt = len / f32(steps);
    var t  = t0 + (0.25 + 0.5 * sample_blue_noise(pixel, params.frame_number)) * dt;

    var acc_ray = vec3<f32>(0.0);
    var acc_mie = vec3<f32>(0.0);

    // Scalar optical depths along the view (expand to RGB on use)
    var tauR = 0.0; var tauM = 0.0; var tauA = 0.0;

    for (var i: i32 = 0; i < steps; i = i + 1) {
        let sp = pos_rel + dir * t;
        let h  = length(sp) - atmo.center_radius.w; // height above planet

        let dR = exp(-h * invHR);
        let dM = exp(-h * invHM);
        let dA = absorption_profile_density(atmo, h);

        tauR += dR * dt;
        tauM += dM * dt;
        tauA += dA * dt;

        var TrL = sample_transmittance_lut(atmo, atmo.center_radius.xyz + sp, light_dir);
        let pl = ray_sphere_intersect(sp, light_dir, atmo.center_radius.w);
        if (pl.y > 0.0 && pl.x > 0.0) {
            TrL = vec3<f32>(0.0);
        }

        let Tview = exp(-(atmo.ray_beta.xyz * vec3<f32>(tauR)
        + atmo.mie_beta.xyz * vec3<f32>(tauM)
        + atmo.absorption_beta.xyz * vec3<f32>(tauA)));

        acc_ray += dR * Tview * TrL * dt;
        acc_mie += dM * Tview * TrL * dt;

        if (Tview.x + Tview.y + Tview.z < T_EARLY_OUT) { break; }

        t += dt;
    }

    let multi_scatter = estimate_multi_scattering(atmo, tauR, tauM, tauA, dir, light_dir, light_intensity);

    let single_scatter = phase_ray * atmo.ray_beta.xyz * acc_ray
    + phase_mie * atmo.mie_beta.xyz * acc_mie;

    let trans = exp(-(atmo.ray_beta.xyz * vec3<f32>(tauR)
    + atmo.mie_beta.xyz * vec3<f32>(tauM)
    + atmo.absorption_beta.xyz * vec3<f32>(tauA)));

    return Scattering(single_scatter * light_intensity + multi_scatter, trans);
}

fn apply_atmosphere(origin: vec3<f32>, dir: vec3<f32>, max_t: f32, background: vec3<f32>) -> vec3<f32> {
    if (params.atmosphere == 0u || params.atmo_count == 0u) { return background; }

    let sun_dir = normalize(-params.dir_light_dir.xyz);
    let sun_I   = params.dir_light_color.xyz * params.dir_light_dir.w;

    // Default to the precomputed LUT path for production rendering. Keep the
    // previous inline marcher behind atmosphere_mode == 1u so visual A/B tests
    // can compare the LUTs against the high-cost integration path.
    if (params.atmosphere_mode == 0u) {
        var lut_col: vec3<f32>;
        if (max_t >= 1e9) {
            lut_col = sample_sky_view_lut(dir);
        } else {
            lut_col = sample_aerial_perspective_lut(dir, max_t, background);
        }

        // Preserve the explicit solar disc/glow used by the inline path; the
        // LUTs model atmospheric radiance, while this keeps directional-light
        // visibility consistent for sky rays during A/B testing.
        let sun_cos = dot(dir, sun_dir);
        let sun_radius = max(params.atmosphere_sun_controls.x, 1e-5);
        let sun_edge = cos(sun_radius);
        if (max_t >= 1e9 && sun_cos > sun_edge) {
            let glow = (sun_cos - sun_edge) / max(1.0 - sun_edge, 1e-5);
            lut_col += sun_I * glow * max(params.atmosphere_sun_controls.y, 0.0);
        }
        return lut_col;
    }

    // Find atmospheres intersected (small N)
    var count: u32 = 0u;
    var tvals: array<f32, MAX_ATMOSPHERES>;
    var idx:   array<u32, MAX_ATMOSPHERES>;
    for (var i: u32 = 0u; i < params.atmo_count; i = i + 1u) {
        let atmo = params.atmos[i];
        let center = atmo.center_radius.xyz;
        let pos = origin - center;
        let rl = ray_sphere_intersect(pos, dir, atmo.atmo_g_height.x);
        let t0 = max(rl.x, 0.0);
        let t1 = min(rl.y, max_t);
        if (t0 <= t1) { tvals[count] = t0; idx[count] = i; count = count + 1u; }
    }
    // tiny selection sort
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        var k = i; var j = i + 1u;
        while (j < count) { if (tvals[j] < tvals[k]) { k = j; } j = j + 1u; }
        if (k != i) {
            let tt = tvals[i]; tvals[i] = tvals[k]; tvals[k] = tt;
            let ii = idx[i];   idx[i]   = idx[k];   idx[k]   = ii;
        }
    }

    var col   = vec3<f32>(0.0);
    var trans = vec3<f32>(1.0);

    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let atmo = params.atmos[idx[i]];

        // Sun
        let sc = calculate_scattering(atmo, origin, dir, max_t, sun_dir, sun_I, vec2<u32>(0u));
        col += trans * sc.color;

        // A few emissive lights (bounded)
        let max_lights: u32 = min(MAX_ATMO_LIGHTS, light_header.count);
        for (var l: u32 = 0u; l < max_lights; l = l + 1u) {
            let obj_idx = light_indices[l];
            let obj = objects[obj_idx];
            let mat = materials[obj.material_index];
            if (mat.emissiveStrength > 0.0) {
                let ldir = normalize(obj.position - origin);
                var lintensity = mat.baseColorFactor.rgb * mat.emissiveStrength;
                if (obj.is_cube > 0u) {
                    let size = obj.size * obj.scale;
                    let area = 2.0 * (size.x * size.y + size.y * size.z + size.z * size.x);
                    lintensity *= area;
                } else {
                    let r = obj.radius * obj.scale.x;
                    let area = 4.0 * PI * r * r;
                    lintensity *= area;
                }
                let sc_l = calculate_scattering(atmo, origin, dir, max_t, ldir, lintensity, vec2<u32>(0u));
                col += trans * sc_l.color;
            }
        }

        trans *= sc.transmittance;
        if (trans.x + trans.y + trans.z < T_EARLY_OUT) { break; }
    }

    col += trans * background;

    // Sun glow only for sky rays
    let sun_cos = dot(dir, sun_dir);
    let sun_radius = max(params.atmosphere_sun_controls.x, 1e-5);
    let sun_edge = cos(sun_radius);
    if (max_t >= 1e9 && sun_cos > sun_edge) {
        let glow = (sun_cos - sun_edge) / max(1.0 - sun_edge, 1e-5);
        col += trans * sun_I * glow * max(params.atmosphere_sun_controls.y, 0.0);
    }
    return col;
}

fn quat_normalize(q: vec4<f32>) -> vec4<f32> { return q * inverseSqrt(dot(q, q)); }
fn quat_conjugate(q: vec4<f32>) -> vec4<f32> { return vec4<f32>(-q.xyz, q.w); }
fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let nq = quat_normalize(q);
    return v + 2.0 * cross(nq.xyz, cross(nq.xyz, v) + nq.w * v);
}

fn sample_ggx(normal: vec3<f32>, roughness: f32, rng: ptr<function, u32>) -> vec3<f32> {
    if (roughness <= 0.0) { return normal; }
    let a = roughness * roughness;
    let r1 = rand(rng);
    let r2 = rand(rng);
    let phi = 2.0 * PI * r1;
    let cos_phi = cos(phi);
    let sin_phi = sin(phi);
    let cos_theta = sqrt((1.0 - r2) / (1.0 + (a * a - 1.0) * r2));
    let sin_theta = sqrt(max(0.0, 1.0 - cos_theta * cos_theta));
    let h_local = vec3<f32>(cos_phi * sin_theta, sin_phi * sin_theta, cos_theta);
    var up = vec3<f32>(0.0, 0.0, 1.0);
    if (abs(normal.z) >= 0.999) { up = vec3<f32>(1.0, 0.0, 0.0); }
    let tangent_x = normalize(cross(up, normal));
    let tangent_y = cross(normal, tangent_x);
    return normalize(tangent_x * h_local.x + tangent_y * h_local.y + normal * h_local.z);
}

// -----------------------------
// Intersections
// -----------------------------
// Returns the first positive intersection distance, even if the ray starts inside.
fn sphere_intersect(origin: vec3<f32>, dir: vec3<f32>, center: vec3<f32>, radius: f32) -> f32 {
    let oc = origin - center;
    let b  = dot(oc, dir);
    let c  = dot(oc, oc) - radius * radius;
    let h  = b * b - c;
    if (h < 0.0) { return NO_HIT; }

    let s   = sqrt(h);
    let t0  = -b - s;     // near
    let t1  = -b + s;     // far
    let eps = max(radius * 1e-5, 1e-4);

    if (t0 > eps) { return t0; }
    if (t1 > eps) { return t1; }  // when starting inside
    return NO_HIT;
}

struct BoxHit { t: f32, n: vec3<f32>, uv: vec2<f32>, }

fn cube_hit(
    origin: vec3<f32>, dir: vec3<f32>,
    pos: vec3<f32>, size: vec3<f32>, orient: vec4<f32>, scale: vec3<f32>
) -> BoxHit {
    let qn = quat_normalize(orient);
    let inv_q = quat_conjugate(qn);

    let oL = quat_rotate(inv_q, origin - pos);
    let dL = quat_rotate(inv_q, dir);

    let half = (size * scale) * 0.5;

    // tiny inflation to avoid edge cracks
    let grow = 1e-4 * max(half.x, max(half.y, half.z));
    let bmin = -half - vec3<f32>(grow);
    let bmax =  half + vec3<f32>(grow);

    let safe = vec3<f32>(1e30);
    let inv_d = select(1.0 / dL, safe, abs(dL) < vec3<f32>(1e-8));

    let tx0 = (bmin.x - oL.x) * inv_d.x; let tx1 = (bmax.x - oL.x) * inv_d.x;
    let ty0 = (bmin.y - oL.y) * inv_d.y; let ty1 = (bmax.y - oL.y) * inv_d.y;
    let tz0 = (bmin.z - oL.z) * inv_d.z; let tz1 = (bmax.z - oL.z) * inv_d.z;

    let tminx = min(tx0, tx1); let tmaxx = max(tx0, tx1);
    let tminy = min(ty0, ty1); let tmaxy = max(ty0, ty1);
    let tminz = min(tz0, tz1); let tmaxz = max(tz0, tz1);

    let t_enter = max(max(tminx, tminy), tminz);
    let t_exit  = min(min(tmaxx, tmaxy), tmaxz);

    if (t_exit < max(t_enter, 0.0)) {
        return BoxHit(1e20, vec3<f32>(0.0, 0.0, 1.0), vec2<f32>(0.0));
    }

    // inside: take exit distance
    let t_hit = select(t_enter, t_exit, t_enter < 0.0);

    // choose face stably
    let pL = oL + dL * t_hit;
    let ax = abs(pL.x) - half.x;
    let ay = abs(pL.y) - half.y;
    let az = abs(pL.z) - half.z;

    var nL = vec3<f32>(0.0);
    var uv = vec2<f32>(0.0);
    if (ax >= ay && ax >= az) {
        nL = vec3<f32>(sign(pL.x), 0.0, 0.0);
        uv = vec2<f32>(pL.z / (half.z * 2.0) + 0.5, pL.y / (half.y * 2.0) + 0.5);
        if (nL.x > 0.0) { uv.x = 1.0 - uv.x; }
    } else if (ay >= ax && ay >= az) {
        nL = vec3<f32>(0.0, sign(pL.y), 0.0);
        uv = vec2<f32>(pL.x / (half.x * 2.0) + 0.5, pL.z / (half.z * 2.0) + 0.5);
        if (nL.y < 0.0) { uv.y = 1.0 - uv.y; }
    } else {
        nL = vec3<f32>(0.0, 0.0, sign(pL.z));
        uv = vec2<f32>(pL.x / (half.x * 2.0) + 0.5, pL.y / (half.y * 2.0) + 0.5);
        if (nL.z < 0.0) { uv.x = 1.0 - uv.x; }
    }

    // fix for non-uniform scale
    let nW = normalize(quat_rotate(qn, nL / scale));
    return BoxHit(t_hit, nW, uv);
}

// SDF helpers (GI)
fn sdf_cube(p: vec3<f32>, pos: vec3<f32>, size: vec3<f32>, orient: vec4<f32>, scale: vec3<f32>) -> f32 {
    let inv_q = quat_conjugate(quat_normalize(orient));
    let lp = quat_rotate(inv_q, p - pos);
    let d = abs(lp) - (size * scale) * 0.5;
    let out_dist = length(max(d, vec3<f32>(0.0)));
    let in_dist = min(max(d.x, max(d.y, d.z)), 0.0);
    return out_dist + in_dist;
}
fn sdf_sphere(p: vec3<f32>, pos: vec3<f32>, r: f32) -> f32 { return length(p - pos) - r; }
fn object_sdf(p: vec3<f32>, obj: Object) -> f32 {
    if (obj.is_cube > 0u) { return sdf_cube(p, obj.position, obj.size, obj.orientation, obj.scale); }
    return sdf_sphere(p, obj.position, obj.radius);
}

// GI cone sampling
const SDF_MIN: vec3<f32> = vec3<f32>(-10.0, -10.0, -10.0);
const SDF_SIZE: vec3<f32> = vec3<f32>(20.0, 20.0, 20.0);
fn sdf_uv(p: vec3<f32>) -> vec3<f32> {
    let uv = (p - SDF_MIN) / SDF_SIZE;
    return clamp(uv, vec3<f32>(0.01), vec3<f32>(0.99));
}
fn sample_radiance_cone(origin: vec3<f32>, dir: vec3<f32>, step: f32) -> vec3<f32> {
    var t = step;
    var col = vec3<f32>(0.0);
    let dims = vec3<f32>(textureDimensions(gi_sdf, 0));
    let voxel = SDF_SIZE / dims;
    let voxel_size = max(voxel.x, max(voxel.y, voxel.z));
    let max_mip = f32(textureNumLevels(gi_sdf) - 1u);
    let half_angle = 0.261799f; // 15 deg
    for (var i: u32 = 0u; i < 8u; i = i + 1u) {
        let pos = origin + dir * t;
        let uv = sdf_uv(pos);
        let cone_r = t * tan(half_angle);
        let cone_d = cone_r * 2.0;
        let lod = clamp(log2(cone_d / voxel_size), 0.0, max_mip);
        let d = textureSampleLevel(gi_sdf, gi_sampler, uv, lod).x;
        if (d <= 0.0 || d > 2.0) { break; }
        let rad = textureSampleLevel(gi_radiance, gi_sampler, uv, lod).xyz;
        col += rad * exp(-d * d * 5.0);
        t += step * 1.5;
    }
    return col / 8.0;
}
fn random_cone_offset(state: ptr<function, u32>) -> vec3<f32> { return random_in_unit_sphere(state); }

fn trace_gi(hit_pos: vec3<f32>, normal: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    if (gi_params.quality == 3u /*OFF*/) { return vec3<f32>(0.0); }
    if (gi_params.quality == 2u /*LOW*/) { return textureSampleLevel(lightmap, gi_sampler, hit_pos.xy, 0.0).rgb; }

    var cones: u32 = 6u;
    if (gi_params.quality == 1u /*HIGH*/) { cones = 3u; }
    let dims = vec3<f32>(textureDimensions(gi_sdf, 0));
    let voxel = SDF_SIZE / dims;
    let voxel_size = max(voxel.x, max(voxel.y, voxel.z));
    let start = hit_pos + normal * max(voxel_size, 0.01);
    var gi = vec3<f32>(0.0);
    for (var c: u32 = 0u; c < cones; c = c + 1u) {
        let dir = normalize(normal + random_cone_offset(rng));
        gi += sample_radiance_cone(start, dir, voxel_size);
    }
    return gi / f32(cones);
}

fn path_trace_gi(p: vec3<f32>, n: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    let origin = p + n * max(params.min_ray_offset, DEFAULT_MIN_RAY_OFFSET);
    let dir = normalize(n + random_cone_offset(rng));
    return trace_ray_no_gi(origin, dir, 0, rng).color;
}
fn sample_diffuse_gi(p: vec3<f32>, n: vec3<f32>, rng: ptr<function, u32>) -> vec3<f32> {
    if (gi_params.mode == 1u) { return path_trace_gi(p, n, rng); }
    return trace_gi(p, n, rng);
}

// -----------------------------
// RR / Tri / AABB / Mesh
// -----------------------------
const RR_MIN_PROB: f32 = 0.1;
fn russian_roulette(depth: i32, roughness: f32, rng: ptr<function, u32>) -> f32 {
    if (depth <= 0) { return 1.0; }
    let rr_prob = max(RR_MIN_PROB, 1.0 - roughness);
    if (rand(rng) > rr_prob) { return 0.0; }
    return 1.0 / rr_prob;
}

struct TriHit { t: f32, u: f32, v: f32, };
fn tri_intersect(origin: vec3<f32>, dir: vec3<f32>, v0: vec3<f32>, e1: vec3<f32>, e2: vec3<f32>) -> TriHit {
    let h = cross(dir, e2);
    let a = dot(e1, h);
    let area2 = length(cross(e1, e2));
    let eps = max(area2 * 1e-8, 1e-7);
    if (abs(a) < eps) { return TriHit(1e20, 0.0, 0.0); }
    let f = 1.0 / a;
    let s = origin - v0;
    let u = f * dot(s, h);
    if (u < -eps || u > 1.0 + eps) { return TriHit(1e20, 0.0, 0.0); }
    let q = cross(s, e1);
    let v = f * dot(dir, q);
    if (v < -eps || u + v > 1.0 + eps) { return TriHit(1e20, 0.0, 0.0); }
    let t = f * dot(e2, q);
    if (t <= max(1e-5, 1e-6 * (length(e1) + length(e2)))) { return TriHit(1e20, 0.0, 0.0); }
    return TriHit(t, u, v);
}

fn aabb_hit(o: vec3<f32>, d: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>, tmax_cap: f32) -> bool {
    let ext = bmax - bmin;
    let eps = max(max(ext.x, max(ext.y, ext.z)) * 1e-4, 1e-5);
    let bminE = bmin - vec3<f32>(eps);
    let bmaxE = bmax + vec3<f32>(eps);

    let c = 0.5 * (bminE + bmaxE);
    let oC = o - c;
    let bminC = bminE - c;
    let bmaxC = bmaxE - c;

    let safe = vec3<f32>(1e30);
    let inv_d = select(1.0 / d, safe, abs(d) < vec3<f32>(1e-8));

    let t0 = (bminC - oC) * inv_d;
    let t1 = (bmaxC - oC) * inv_d;

    let tmin = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
    let tmax = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
    return (tmax >= max(tmin, 0.0)) && (tmin <= tmax_cap);
}

struct MeshHit { n: vec3<f32>, t: f32, tri: u32, uv: vec2<f32>, bvh_visits: u32, tri_tests: u32, terminated: u32, };

fn mesh_intersect(origin: vec3<f32>, dir: vec3<f32>, obj: Object) -> MeshHit {
    var best_t = NO_HIT;
    var best_n = vec3<f32>(0.0, 0.0, 1.0);
    var best_tri: u32 = 0xffffffffu;
    var best_uv = vec2<f32>(0.0);

    var stack: array<i32, 256>;
    var sp: i32 = 0;
    let root = i32(obj.tri_bvh_start);
    if (root < 0) {
        return MeshHit(vec3<f32>(0.0, 0.0, 1.0), NO_HIT, 0xffffffffu, vec2<f32>(0.0), 0u, 0u, 0u);
    }
    stack[sp] = root; sp = sp + 1;

    var steps: u32 = 0u; var bvh_visits: u32 = 0u; var tri_tests: u32 = 0u; var terminated: u32 = 0u;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u;
        if (steps > max(params.max_traversal_steps, 1u)) { terminated = 1u; break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tri_node(ni)) { continue; }
        let node = tri_bvh_nodes[ni];
        bvh_visits = bvh_visits + 1u;
        if (!aabb_hit(origin, dir, node.bmin.xyz, node.bmax.xyz, best_t)) { continue; }
        let c0 = node.child_tri.x; let c1 = node.child_tri.y;
        if (c0 < 0 && c1 < 0) {
            let ti = u32(node.child_tri.z);
            if (in_bounds_tri(ti)) {
                let tri = triangles[ti];
                tri_tests = tri_tests + 1u;
                let hit = tri_intersect(origin, dir, tri.v0, tri.e1, tri.e2);
                if (hit.t < best_t) {
                    best_t = hit.t;
                    let w0 = 1.0 - hit.u - hit.v;
                    best_n = normalize(w0 * tri.n0 + hit.u * tri.n1 + hit.v * tri.n2);
                    best_tri = ti;
                    best_uv = tri.uv0 + tri.duv1 * hit.u + tri.duv2 * hit.v;
                }
            }
        } else {
            if (c0 >= 0 && sp < 256) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 256) { stack[sp] = c1; sp = sp + 1; }
        }
    }

    if (best_t >= 1e19) {
        return MeshHit(vec3<f32>(0.0, 0.0, 1.0), NO_HIT, 0xffffffffu, vec2<f32>(0.0), bvh_visits, tri_tests, terminated);
    }

    return MeshHit(best_n, best_t, best_tri, best_uv, bvh_visits, tri_tests, terminated);
}



fn mesh_any_hit(origin: vec3<f32>, dir: vec3<f32>, obj: Object, t_cap: f32) -> bool {
    var stack: array<i32, 256>;
    var sp: i32 = 0;
    let root = i32(obj.tri_bvh_start);
    if (root < 0) { return false; }
    stack[sp] = root; sp = sp + 1;

    loop {
        if (sp == 0) { break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tri_node(ni)) { continue; }
        let node = tri_bvh_nodes[ni];
        if (!aabb_hit(origin, dir, node.bmin.xyz, node.bmax.xyz, t_cap)) { continue; }
        let c0 = node.child_tri.x; let c1 = node.child_tri.y;
        if (c0 < 0 && c1 < 0) {
            let ti = u32(node.child_tri.z);
            if (in_bounds_tri(ti)) {
                let tri = triangles[ti];
                let hit = tri_intersect(origin, dir, tri.v0, tri.e1, tri.e2);
                if (hit.t < t_cap) {
                    var mat_idx: u32 = obj.material_index;
                    mat_idx = tri.material_index;
                    let mat = materials[mat_idx];
                    var a = mat.baseColorFactor.w;
                    if (mat.baseColorTex != 0u) {
                        let uv = tri.uv0 + tri.duv1 * hit.u + tri.duv2 * hit.v;
                        a = a * textureSampleLevel(textures[mat.baseColorTex], tex_sampler, uv, 0.0).a;
                    }
                    if (obj.is_glass > 0u || a >= 0.5) { return true; }
                }
            }
        } else {
            if (c0 >= 0 && sp < 256) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 256) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return false;
}

// -----------------------------
// TLAS traversal
// -----------------------------
fn tlas_aabb_hit(o: vec3<f32>, d: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>, t_cap: f32) -> bool {
    let inv_d = 1.0 / d;
    let t0 = (bmin - o) * inv_d;
    let t1 = (bmax - o) * inv_d;
    let tsm = min(t0, t1);
    let tbig = max(t0, t1);
    let t_near = max(max(tsm.x, tsm.y), max(tsm.z, 0.0));
    let t_far = min(min(tbig.x, tbig.y), tbig.z);
    return t_far >= t_near && t_near < t_cap;
}

fn aabb_near(o: vec3<f32>, d: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>, t_cap: f32) -> f32 {
    let inv_d = 1.0 / d;
    let t0 = (bmin - o) * inv_d;
    let t1 = (bmax - o) * inv_d;
    let tsm = min(t0, t1);
    let tbig = max(t0, t1);
    let t_near = max(max(tsm.x, tsm.y), max(tsm.z, 0.0));
    let t_far = min(min(tbig.x, tbig.y), tbig.z);
    return select(1e20, t_near, t_far >= t_near && t_near < t_cap);
}

fn in_bounds_tlas(i: u32) -> bool { return i < params.total_bvh_nodes; }
fn in_bounds_tri_node(i: u32) -> bool { return i < params.total_tri_bvh_nodes; }
fn in_bounds_tri(i: u32) -> bool { return i < params.total_triangles; }

// Alpha cutout (never cut glass)
fn hit_alpha(obj_idx: u32, tri_idx: u32, uv: vec2<f32>) -> f32 {
    if (objects[obj_idx].is_glass > 0u) { return 1.0; }
    var mat_idx: u32 = objects[obj_idx].material_index;
    if (tri_idx != 0xffffffffu) {
        let tri = triangles[tri_idx]; mat_idx = tri.material_index;
    }
    let mat = materials[mat_idx];
    var a = mat.baseColorFactor.w;
    if (mat.baseColorTex != 0u && tri_idx != 0xffffffffu) {
        let tex = textureSampleLevel(textures[mat.baseColorTex], tex_sampler, uv, 0.0);
        a = a * tex.a;
    }
    return select(0.0, 1.0, a >= 0.5);
}

struct ObjHit { t: f32, n: vec3<f32>, idx: i32, tri: u32, uv: vec2<f32>, tlas_visits: u32, blas_visits: u32, tri_tests: u32, terminated: u32, };

fn object_tlas_intersect(o: vec3<f32>, d: vec3<f32>, skip: i32) -> ObjHit {
    var best = ObjHit(1e20, vec3<f32>(0.0, 0.0, 1.0), -1, 0xffffffffu, vec2<f32>(0.0), 0u, 0u, 0u, 0u);
    if (params.total_bvh_nodes == 0u) { return best; }
    var stack: array<i32, 256>; var sp: i32 = 0;
    let root: i32 = 0; stack[sp] = root; sp = sp + 1;
    var steps: u32 = 0u;

    loop {
        if (sp == 0) { break; }
        steps = steps + 1u;
        if (steps > max(params.max_traversal_steps, 1u)) { best.terminated = 1u; break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tlas(ni)) { continue; }
        let node = bvh_nodes[ni];
        best.tlas_visits = best.tlas_visits + 1u;
        if (!tlas_aabb_hit(o, d, node.bmin.xyz, node.bmax.xyz, best.t)) { continue; }

        let c0 = node.child_object.x; let c1 = node.child_object.y;
        if (c0 < 0 && c1 < 0) {
            let i0 = node.child_object.z; let i1 = node.child_object.w;

            if (i0 >= 0) {
                let i = u32(i0);
                if (i32(i) != skip && i < u32(params.num_objects)) {
                    let oref = objects[i];
                    var t = 1e20; var n = vec3<f32>(0.0); var tri_idx: u32 = 0xffffffffu; var uv = vec2<f32>(0.0);

                    if (oref.is_mesh > 0u) {
                        let res = mesh_intersect(o, d, oref);
                        t = res.t; n = res.n; tri_idx = res.tri; uv = res.uv;
                        best.blas_visits += res.bvh_visits; best.tri_tests += res.tri_tests; best.terminated = max(best.terminated, res.terminated);
                    } else if (oref.is_cube > 0u) {
                        let bh = cube_hit(o, d, oref.position, oref.size, oref.orientation, oref.scale);
                        t = bh.t;
                        if (t < 1e20) { n = bh.n; uv = bh.uv; }
                    } else {
                        t = sphere_intersect(o, d, oref.position, oref.radius);
                        if (t < 1e20) {
                            let hp = o + d * t;
                            n = normalize(hp - oref.position);
                            let u = 0.5 + atan2(n.z, n.x) / TAU;
                            let v = 0.5 - asin(n.y) / PI;
                            uv = vec2<f32>(u, v);
                        }
                    }

                    if (t < best.t) {
                        if (dot(n, d) > 0.0) { n = -n; }
                        best.t = t; best.n = n; best.idx = i32(i); best.tri = tri_idx; best.uv = uv;
                    }
                }
            }

            if (i1 >= 0) {
                let i = u32(i1);
                if (i32(i) != skip && i < u32(params.num_objects)) {
                    let oref = objects[i];
                    var t = 1e20; var n = vec3<f32>(0.0); var tri_idx: u32 = 0xffffffffu; var uv = vec2<f32>(0.0);

                    if (oref.is_mesh > 0u) {
                        let res = mesh_intersect(o, d, oref);
                        t = res.t; n = res.n; tri_idx = res.tri; uv = res.uv;
                        best.blas_visits += res.bvh_visits; best.tri_tests += res.tri_tests; best.terminated = max(best.terminated, res.terminated);
                    } else if (oref.is_cube > 0u) {
                        let bh = cube_hit(o, d, oref.position, oref.size, oref.orientation, oref.scale);
                        t = bh.t;
                        if (t < 1e20) { n = bh.n; uv = bh.uv; }
                    } else {
                        t = sphere_intersect(o, d, oref.position, oref.radius);
                        if (t < 1e20) {
                            let hp = o + d * t;
                            n = normalize(hp - oref.position);
                            let u = 0.5 + atan2(n.z, n.x) / TAU;
                            let v = 0.5 - asin(n.y) / PI;
                            uv = vec2<f32>(u, v);
                        }
                    }

                    if (t < best.t) {
                        if (dot(n, d) > 0.0) { n = -n; }
                        best.t = t; best.n = n; best.idx = i32(i); best.tri = tri_idx; best.uv = uv;
                    }
                }
            }
        } else {
            var n0 = 1e20; var n1 = 1e20;
            if (c0 >= 0 && in_bounds_tlas(u32(c0))) { let cn = bvh_nodes[u32(c0)]; n0 = aabb_near(o, d, cn.bmin.xyz, cn.bmax.xyz, best.t); }
            if (c1 >= 0 && in_bounds_tlas(u32(c1))) { let cn = bvh_nodes[u32(c1)]; n1 = aabb_near(o, d, cn.bmin.xyz, cn.bmax.xyz, best.t); }
            if (n0 < n1) {
                if (n1 < 1e19 && sp < 256) { stack[sp] = c1; sp = sp + 1; }
                if (n0 < 1e19 && sp < 256) { stack[sp] = c0; sp = sp + 1; }
            } else {
                if (n0 < 1e19 && sp < 256) { stack[sp] = c0; sp = sp + 1; }
                if (n1 < 1e19 && sp < 256) { stack[sp] = c1; sp = sp + 1; }
            }
        }
    }
    return best;
}

fn object_tlas_any_hit(o: vec3<f32>, d: vec3<f32>, t_cap: f32, skip_a: u32, skip_b: u32) -> bool {
    if (params.total_bvh_nodes == 0u) { return false; }
    var stack: array<i32, 256>; var sp: i32 = 0;
    let root: i32 = 0; stack[sp] = root; sp = sp + 1;

    loop {
        if (sp == 0) { break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tlas(ni)) { continue; }
        let node = bvh_nodes[ni];
        if (!tlas_aabb_hit(o, d, node.bmin.xyz, node.bmax.xyz, t_cap)) { continue; }

        let c0 = node.child_object.x; let c1 = node.child_object.y;
        if (c0 < 0 && c1 < 0) {
            let i0 = node.child_object.z; let i1 = node.child_object.w;

            if (i0 >= 0) {
                let i = u32(i0);
                if (i != skip_a && i != skip_b && i < u32(params.num_objects)) {
                    let oref = objects[i];
                    var t = 1e20; var tri_idx: u32 = 0xffffffffu; var uv = vec2<f32>(0.0);

                    if (oref.is_mesh > 0u) {
                        if (mesh_any_hit(o, d, oref, t_cap)) { return true; }
                        t = 1e20;
                    } else if (oref.is_cube > 0u) {
                        t = cube_hit(o, d, oref.position, oref.size, oref.orientation, oref.scale).t;
                    } else {
                        t = sphere_intersect(o, d, oref.position, oref.radius);
                    }

                    if (t < t_cap) {
                        let a = hit_alpha(i, tri_idx, uv);
                        if (a > 0.001) { return true; }
                    }
                }
            }

            if (i1 >= 0) {
                let i = u32(i1);
                if (i != skip_a && i != skip_b && i < u32(params.num_objects)) {
                    let oref = objects[i];
                    var t = 1e20; var tri_idx: u32 = 0xffffffffu; var uv = vec2<f32>(0.0);

                    if (oref.is_mesh > 0u) {
                        if (mesh_any_hit(o, d, oref, t_cap)) { return true; }
                        t = 1e20;
                    } else if (oref.is_cube > 0u) {
                        t = cube_hit(o, d, oref.position, oref.size, oref.orientation, oref.scale).t;
                    } else {
                        t = sphere_intersect(o, d, oref.position, oref.radius);
                    }

                    if (t < t_cap) {
                        let a = hit_alpha(i, tri_idx, uv);
                        if (a > 0.001) { return true; }
                    }
                }
            }
        } else {
            if (c0 >= 0 && sp < 256) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 256) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return false;
}

fn is_visible(origin: vec3<f32>, end_pos: vec3<f32>, skip_a: u32, skip_b: u32) -> bool {
    var dir = end_pos - origin;
    let dist = min(length(dir), params.shadow_max_distance);
    dir = dir / dist;
    return !object_tlas_any_hit(origin, dir, min(dist - max(params.min_ray_offset, DEFAULT_MIN_RAY_OFFSET), min(params.rt_shadow_ray_t_max, params.shadow_max_distance)), skip_a, skip_b);
}

fn rt_shadow_mode_enabled() -> bool {
    return params.raytraced_shadows_enabled != 0u &&
        params.shadow_quality != SHADOW_QUALITY_OFF &&
        params.shadow_mode != SHADOW_MODE_NONE &&
        params.shadow_mode != SHADOW_MODE_RASTER;
}

fn soft_rt_shadow_enabled(light_radius: f32, dist: f32) -> bool {
    if (params.shadow_mode == SHADOW_MODE_RT_HARD) { return false; }
    if (params.shadow_mode == SHADOW_MODE_RT_SOFT) { return true; }
    // Hybrid: keep shadow maps as the default and only spend soft RT rays on
    // nearby/contact-important lights large enough to produce a visible penumbra.
    let angular_radius = light_radius / max(dist, 1.0);
    return params.shadow_mode == SHADOW_MODE_HYBRID &&
        dist <= params.max_rt_shadow_distance &&
        angular_radius >= params.min_soft_shadow_radius;
}

// -----------------------------
// Lighting / Shading
// -----------------------------
fn soft_shadow(origin: vec3<f32>, normal: vec3<f32>, light: Object, light_idx: u32, rng: ptr<function, u32>) -> f32 {
    let lm = materials[light.material_index];
    if (lm.emissiveStrength <= 0.0) { return 1.0; }
    var vis: f32 = 0.0;
    let light_dist = length(light.position - origin);
    let light_rt_max = min(params.max_rt_shadow_distance, light.max_shadow_distance);
    let hybrid_selected = params.shadow_mode != SHADOW_MODE_HYBRID || light.casts_raytraced_shadow != 0u || light.shadow_importance >= 0.75;
    if (!rt_shadow_mode_enabled() || !hybrid_selected || light_dist > light_rt_max) { return 1.0; }
    if (!soft_rt_shadow_enabled(max(light.radius, max(max(light.size.x * light.scale.x, light.size.y * light.scale.y), light.size.z * light.scale.z) * 0.5), light_dist)) {
        return select(0.0, 1.0, is_visible(origin, light.position, 0xffffffffu, light_idx));
    }
    let requested = max(1u, max(u32(params.light_samples), params.emissive_shadow_samples));
    var quality_cap = MAX_LIGHT_SAMPLES;
    if (params.shadow_quality <= SHADOW_QUALITY_LOW) { quality_cap = 1u; }
    if (params.shadow_quality == SHADOW_QUALITY_MEDIUM) { quality_cap = 2u; }
    let ray_cap = select(MAX_LIGHT_SAMPLES, max(1u, params.max_shadow_rays), params.max_shadow_rays > 0u);
    let samples: u32 = clamp(requested, 1u, min(ray_cap, quality_cap));
    for (var s: u32 = 0u; s < samples; s = s + 1u) {
        var offset = vec3<f32>(0.0);
        if (light.is_cube > 0u) {
            let local = random_cube_surface(rng) * (light.size * light.scale * 0.5);
            offset = quat_rotate(light.orientation, local);
        } else {
            offset = random_in_unit_sphere(rng) * light.radius;
        }
        let targetc = light.position + offset;
        if (is_visible(origin, targetc, 0xffffffffu, light_idx)) { vis = vis + 1.0; }
    }
    return vis / f32(samples);
}

const DIR_SHADOW_RADIUS: f32 = 0.05;
fn dir_soft_shadow(origin: vec3<f32>, dir: vec3<f32>, rng: ptr<function, u32>) -> f32 {
    var vis: f32 = 0.0;
    if (!rt_shadow_mode_enabled()) { return 1.0; }
    let dir_dist = params.max_rt_shadow_distance;
    if (params.shadow_mode == SHADOW_MODE_RT_HARD || params.shadow_quality == SHADOW_QUALITY_LOW || !soft_rt_shadow_enabled(DIR_SHADOW_RADIUS, dir_dist)) {
        return select(0.0, 1.0, is_visible(origin, origin + dir * min(1000.0, params.max_rt_shadow_distance), 0xffffffffu, 0xffffffffu));
    }
    let requested = max(1u, max(u32(params.dir_shadow_samples), params.directional_shadow_samples));
    var quality_cap = MAX_DIR_SHADOW_SAMPLES;
    if (params.shadow_quality == SHADOW_QUALITY_MEDIUM) { quality_cap = 2u; }
    let ray_cap = select(MAX_DIR_SHADOW_SAMPLES, max(1u, params.max_shadow_rays), params.max_shadow_rays > 0u);
    let samples: u32 = clamp(requested, 1u, min(ray_cap, quality_cap));
    for (var s: u32 = 0u; s < samples; s = s + 1u) {
        let jitter = random_in_unit_sphere(rng) * DIR_SHADOW_RADIUS;
        let sdir = normalize(dir + jitter);
        if (is_visible(origin, origin + sdir * min(1000.0, params.max_rt_shadow_distance), 0xffffffffu, 0xffffffffu)) { vis = vis + 1.0; }
    }
    return vis / f32(samples);
}

fn ambient_occlusion(_origin: vec3<f32>, _normal: vec3<f32>, _obj_idx: u32, _rng: ptr<function, u32>) -> f32 {
    return 1.0; // SSAO elsewhere
}

fn default_material_result(hit_point: vec3<f32>, normal: vec3<f32>, _view_dir: vec3<f32>, _uv: vec2<f32>) -> MaterialResult {
    var result: MaterialResult;
    result.base_color = vec3<f32>(1.0, 1.0, 1.0);
    result.normal = normal;
    result.roughness = 1.0;
    result.metallic = 0.0;
    result.emission = vec3<f32>(0.0, 0.0, 0.0);
    result.transparency = 0.0;
    result.transmission = 0.0;
    result.transmission_roughness = 0.0;
    result.ior = 1.5;
    result.subsurface = vec4<f32>(0.0);
    result.clearcoat = vec2<f32>(0.0);
    result.anisotropy = vec2<f32>(0.0);
    result.sheen = vec4<f32>(0.0);
    result.displacement = 0.0;
    return result;
}

fn evaluate_custom_material(
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    uv: vec2<f32>,
    material_id: u32,
) -> MaterialResult {
    // MATERIAL_EVALUATION_PLACEHOLDER
    return default_material_result(hit_point, normal, view_dir, uv);
}

fn evaluate_default_material(hit: vec3<f32>, normal: vec3<f32>, mat: MaterialParams, tri_idx: u32, uv: vec2<f32>) -> MaterialResult {
    var result: MaterialResult;
    var tex = vec3<f32>(1.0, 1.0, 1.0);
    if (mat.baseColorTex != 0u && tri_idx != 0xffffffffu) {
        tex = textureSampleLevel(textures[mat.baseColorTex], tex_sampler, uv, 0.0).rgb;
    }
    result.base_color = mat.baseColorFactor.rgb * tex;
    result.normal = normal;
    result.roughness = mat.roughnessFactor;
    result.metallic = mat.metallicFactor;
    result.emission = mat.emissiveFactor * mat.emissiveStrength;
    result.transparency = 0.0;
    result.transmission = 0.0;
    result.transmission_roughness = 0.0;
    result.ior = mat.ior;
    result.subsurface = vec4<f32>(0.0);
    result.clearcoat = vec2<f32>(0.0);
    result.anisotropy = vec2<f32>(0.0);
    result.sheen = vec4<f32>(0.0);
    result.displacement = 0.0;
    return result;
}

fn shade_base(
    hit: vec3<f32>, normal: vec3<f32>, obj_idx: u32, tri_idx: u32, uv: vec2<f32>, gi: vec3<f32>, rng: ptr<function, u32>
) -> vec4<f32> {
    var mat_idx: u32 = objects[obj_idx].material_index;
    if (tri_idx != 0xffffffffu) { let tri = triangles[tri_idx]; mat_idx = tri.material_index; }
    var mat = materials[mat_idx];

    var material_result: MaterialResult;
    if (mat.has_custom_material != 0u) {
        let view_dir = normalize(params.camera_pos.xyz - hit);
        material_result = evaluate_custom_material(hit, normal, view_dir, uv, mat.custom_material_id);
        // Adopt roughness/metallic/emissive output by the custom shader
        mat.roughnessFactor = material_result.roughness;
        mat.metallicFactor = material_result.metallic;
        let e_strength = max(max(material_result.emission.r, material_result.emission.g), material_result.emission.b);
        if (e_strength > 0.0) {
            mat.emissiveStrength = e_strength;
            mat.emissiveFactor = material_result.emission / e_strength;
        }
    } else {
        material_result = evaluate_default_material(hit, normal, mat, tri_idx, uv);
    }

    let base = material_result.base_color;
    var surface_normal = material_result.normal;

    if (objects[obj_idx].is_shaded == 0u) {
        return vec4<f32>(base, material_result.transparency);
    }

    let sky_tint = params.skycolor.xyz;
    let sky = max(dot(surface_normal, vec3<f32>(0.0, 1.0, 0.0)), 0.0);
    let sky_mix = mix(1.0, sky, params.sky_occlusion);
    var col = base * sky_tint * 0.2 * sky_mix;

    col += gi * base * sky_mix;

    // Add any emissive contribution after potential custom-material edits
    col += mat.emissiveFactor * mat.emissiveStrength;

    let ao = ambient_occlusion(hit + surface_normal * 0.01, surface_normal, obj_idx, rng);

    let light_count: u32 = light_header.count;
    if (light_count > 0u) {
        // Bounded per-pixel reservoir over a tiny candidate set. The CPU keeps
        // light_indices sorted by importance, so many emissive lights do not
        // translate into linear RT shadow work here.
        let candidate_count = min(light_count, MAX_IMPORTANT_PIXEL_LIGHTS);
        let tile_raw = vec2<u32>(floor((hit.xy + vec2<f32>(1024.0)) / 8.0));
        let tile_x = tile_raw.x % 16u;
        let tile_y = tile_raw.y % 16u;
        let start = (tile_x + tile_y * 13u + u32(params.frame_number)) % candidate_count;
        var chosen_idx = 0xffffffffu;
        var chosen_weight = 0.0;
        var weight_sum = 0.0;
        for (var c = 0u; c < MAX_IMPORTANT_PIXEL_LIGHTS; c = c + 1u) {
            if (c >= candidate_count) { break; }
            let list_i = (start + c) % candidate_count;
            let idx = light_indices[list_i];
            if (idx == obj_idx) { continue; }
            let lobj = objects[idx];
            let lm = materials[lobj.material_index];
            if (lm.emissiveStrength <= 0.0) { continue; }
            let to_light = lobj.position - hit;
            let dist2 = max(dot(to_light, to_light), 1.0);
            let n_dot_l = max(dot(surface_normal, normalize(to_light)), 0.0);
            let energy = max(max(lm.baseColorFactor.r, lm.baseColorFactor.g), lm.baseColorFactor.b) * lm.emissiveStrength;
            let radius = max(lobj.radius * max(max(lobj.scale.x, lobj.scale.y), lobj.scale.z), 0.25);
            let screen_influence = clamp(radius * inverseSqrt(dist2), 0.0, 1.0);
            let visibility_relevance = max(lobj.shadow_importance, select(0.35, 1.0, lobj.casts_raytraced_shadow != 0u));
            let w = max(0.0, energy * (0.15 + n_dot_l) * (0.25 + screen_influence) * visibility_relevance / dist2);
            if (w <= 0.0) { continue; }
            weight_sum = weight_sum + w;
            if (rand(rng) * weight_sum <= w) {
                chosen_idx = idx;
                chosen_weight = w;
            }
        }
        if (chosen_idx != 0xffffffffu && chosen_weight > 0.0) {
            if (gi_params.debug_mode == 7u) {
                let h = f32((chosen_idx * 1103515245u + 12345u) & 255u) / 255.0;
                return vec4<f32>(vec3<f32>(h, fract(h * 7.0), fract(h * 13.0)), material_result.transparency);
            }
            let light = objects[chosen_idx];
            let lm = materials[light.material_index];
            let light_pos = light.position;
            var light_target = light_pos;
            var light_area = 1.0;
            if (light.is_cube > 0u) {
                let size = light.size * light.scale;
                light_area = 2.0 * (size.x * size.y + size.y * size.z + size.z * size.x);
                let local = random_cube_surface(rng) * (size * 0.5);
                light_target = light_pos + quat_rotate(light.orientation, local);
            } else {
                let r = light.radius * light.scale.x;
                light_area = 4.0 * PI * r * r;
                let local = random_in_unit_sphere(rng) * r;
                light_target = light_pos + local;
            }
            let ldir = normalize(light_target - hit);
            let diff = max(dot(surface_normal, ldir), 0.0);
            if (diff > 0.0) {
                let visibility = soft_shadow(hit + surface_normal * 0.01, surface_normal, light, chosen_idx, rng);
                if (visibility > 0.0) {
                    let dist = length(light_target - hit);
                    let attenuation = 1.0 / max(dist * dist, 1.0);
                    let mis_weight = weight_sum / chosen_weight;
                    let L = lm.baseColorFactor.rgb * lm.emissiveStrength;
                    col += base * L * diff * visibility * attenuation * mis_weight * light_area;
                }
            }
        }
    }
    if (params.dir_light_dir.w > 0.0) {
        let ldir = normalize(-params.dir_light_dir.xyz);
        let vis = dir_soft_shadow(hit + surface_normal * 0.01, ldir, rng);
        if (vis > 0.0) {
            let diff = max(dot(surface_normal, ldir), 0.0);
            let cloud_shadow = cloud_shadow_transmittance(hit + surface_normal * 0.01, ldir, rand(rng));
            col += base * params.dir_light_color.xyz * diff * params.dir_light_dir.w * vis * cloud_shadow;
        }
    }
    col = col * ao;
    return vec4<f32>(col, material_result.transparency);
}

fn shade(hit: vec3<f32>, normal: vec3<f32>, obj_idx: u32, tri_idx: u32, uv: vec2<f32>, rng: ptr<function, u32>) -> vec4<f32> {
    let gi = sample_diffuse_gi(hit, normal, rng);
    return shade_base(hit, normal, obj_idx, tri_idx, uv, gi, rng);
}
fn shade_no_gi(hit: vec3<f32>, normal: vec3<f32>, obj_idx: u32, tri_idx: u32, uv: vec2<f32>, rng: ptr<function, u32>) -> vec4<f32> {
    return shade_base(hit, normal, obj_idx, tri_idx, uv, vec3<f32>(0.0), rng);
}

// -----------------------------
// Rays
// -----------------------------
struct RayResult { color: vec3<f32>, depth: f32, normal: vec3<f32>, obj_idx: i32, tri_idx: u32, tlas_visits: u32, blas_visits: u32, tri_tests: u32, shadow_rays: u32, reflection_rays: u32, gi_rays: u32, terminated: u32, };
// ---- helpers for size-scaled eps and skipping self ----
fn object_extent(obj: Object) -> f32 {
    let s = obj.size * obj.scale;
    return max(s.x, max(s.y, s.z));
}
fn surf_eps(obj: Object) -> f32 {
    return max(1e-4, object_extent(obj) * 1e-4);
}

fn trace_ray_base(origin: vec3<f32>, dir: vec3<f32>, depth: i32, rng: ptr<function, u32>, skip: i32) -> RayResult {
    var o = origin;
    var s = skip;
    var d = depth;
    var t_total = 0.0;
    var atten = 1.0;
    var col = vec3<f32>(0.0);
    var norm = vec3<f32>(0.0, 0.0, 1.0);
    var obj_idx = -1;
    var tri_idx = 0u;
    let max_ray_distance = select(1e20, params.reflection_max_distance, depth > 0);
    var tlas_visits = 0u; var blas_visits = 0u; var tri_tests = 0u; var terminated = 0u;
    for (var iter: u32 = 0u; iter < MAX_TLAS_ITERS; iter = iter + 1u) {
        if (d >= params.max_bounces) { break; }
        let hit = object_tlas_intersect(o, dir, s);
        tlas_visits += hit.tlas_visits; blas_visits += hit.blas_visits; tri_tests += hit.tri_tests; terminated = max(terminated, hit.terminated);
        if (hit.idx < 0) { break; }
        let alpha = hit_alpha(u32(hit.idx), hit.tri, hit.uv);
        t_total = t_total + hit.t;
        if (t_total > max_ray_distance) { terminated = 1u; break; }
        if (alpha < 0.5) {
            if (iter >= params.max_transparent_surfaces) { terminated = 1u; break; }
            o = o + dir * (hit.t + params.min_ray_offset);
            t_total = t_total + params.min_ray_offset;
            s = -1;
            continue;
        }
        let hit_pos = origin + dir * t_total;
        var shade_res: vec4<f32>;
        if (d == 0) {
            shade_res = shade(hit_pos, hit.n, u32(hit.idx), hit.tri, hit.uv, rng);
        } else {
            shade_res = shade_no_gi(hit_pos, hit.n, u32(hit.idx), hit.tri, hit.uv, rng);
        }
        var surf_col = shade_res.xyz;
        let trans = shade_res.w;
        surf_col = apply_atmosphere(origin, dir, t_total, surf_col);
        col = col + surf_col * atten * (1.0 - trans);
        atten = atten * trans;
        norm = hit.n;
        obj_idx = hit.idx;
        tri_idx = hit.tri;
        if (atten <= 0.0) {
            return RayResult(col, t_total, norm, obj_idx, tri_idx, tlas_visits, blas_visits, tri_tests, 0u, u32(depth > 0), 0u, terminated);
        }
        let obj = objects[u32(hit.idx)];
        if (d >= 1) {
            let mat = materials[obj.material_index];
            let rr = russian_roulette(d, mat.roughnessFactor, rng);
            if (rr == 0.0) {
                return RayResult(col, t_total, norm, obj_idx, tri_idx, tlas_visits, blas_visits, tri_tests, 0u, u32(depth > 0), 0u, terminated);
            }
            atten = atten * rr;
        }
        let eps = max(surf_eps(obj), params.min_ray_offset);
        o = hit_pos + dir * eps;
        t_total = t_total + eps;
        s = hit.idx;
        d = d + 1;
    }
    let sky = apply_atmosphere(origin, dir, 1e9, params.skycolor.xyz);
    col = col + sky * atten;
    return RayResult(col, 1.0, norm, obj_idx, tri_idx, tlas_visits, blas_visits, tri_tests, 0u, u32(depth > 0), 0u, terminated);
}

fn trace_ray_base_no_gi(origin: vec3<f32>, dir: vec3<f32>, depth: i32, rng: ptr<function, u32>, skip: i32) -> RayResult {
    var o = origin;
    var s = skip;
    var d = depth;
    var t_total = 0.0;
    var atten = 1.0;
    var col = vec3<f32>(0.0);
    var norm = vec3<f32>(0.0, 0.0, 1.0);
    var obj_idx = -1;
    var tri_idx = 0u;
    let max_ray_distance = select(params.gi_max_distance, params.reflection_max_distance, depth > 0);
    var tlas_visits = 0u; var blas_visits = 0u; var tri_tests = 0u; var terminated = 0u;
    for (var iter: u32 = 0u; iter < MAX_TLAS_ITERS; iter = iter + 1u) {
        if (d >= params.max_bounces) { break; }
        let hit = object_tlas_intersect(o, dir, s);
        tlas_visits += hit.tlas_visits; blas_visits += hit.blas_visits; tri_tests += hit.tri_tests; terminated = max(terminated, hit.terminated);
        if (hit.idx < 0) { break; }
        let alpha = hit_alpha(u32(hit.idx), hit.tri, hit.uv);
        t_total = t_total + hit.t;
        if (t_total > max_ray_distance) { terminated = 1u; break; }
        if (alpha < 0.5) {
            if (iter >= params.max_transparent_surfaces) { terminated = 1u; break; }
            o = o + dir * (hit.t + params.min_ray_offset);
            t_total = t_total + params.min_ray_offset;
            s = -1;
            continue;
        }
        let hit_pos = origin + dir * t_total;
        let shade_res = shade_no_gi(hit_pos, hit.n, u32(hit.idx), hit.tri, hit.uv, rng);
        var surf_col = shade_res.xyz;
        let trans = shade_res.w;
        surf_col = apply_atmosphere(origin, dir, t_total, surf_col);
        col = col + surf_col * atten * (1.0 - trans);
        atten = atten * trans;
        norm = hit.n;
        obj_idx = hit.idx;
        tri_idx = hit.tri;
        if (atten <= 0.0) {
            return RayResult(col, t_total, norm, obj_idx, tri_idx, tlas_visits, blas_visits, tri_tests, 0u, u32(depth > 0), 0u, terminated);
        }
        let obj = objects[u32(hit.idx)];
        if (d >= 1) {
            let mat = materials[obj.material_index];
            let rr = russian_roulette(d, mat.roughnessFactor, rng);
            if (rr == 0.0) {
                return RayResult(col, t_total, norm, obj_idx, tri_idx, tlas_visits, blas_visits, tri_tests, 0u, u32(depth > 0), 0u, terminated);
            }
            atten = atten * rr;
        }
        let eps = max(surf_eps(obj), params.min_ray_offset);
        o = hit_pos + dir * eps;
        t_total = t_total + eps;
        s = hit.idx;
        d = d + 1;
    }
    let sky = apply_atmosphere(origin, dir, 1e9, params.skycolor.xyz);
    col = col + sky * atten;
    return RayResult(col, 1.0, norm, obj_idx, tri_idx, tlas_visits, blas_visits, tri_tests, 0u, u32(depth > 0), 0u, terminated);
}

fn trace_ray(origin: vec3<f32>, dir: vec3<f32>, depth: i32, rng: ptr<function, u32>) -> RayResult {
    return trace_ray_base(origin, dir, depth, rng, -1);
}

fn trace_ray_no_gi(origin: vec3<f32>, dir: vec3<f32>, depth: i32, rng: ptr<function, u32>) -> RayResult {
    return trace_ray_base_no_gi(origin, dir, depth, rng, -1);
}

fn trace_ray_skip(origin: vec3<f32>, dir: vec3<f32>, depth: i32, rng: ptr<function, u32>, skip: i32) -> RayResult {
    return trace_ray_base(origin, dir, depth, rng, skip);
}

fn trace_ray_skip_no_gi(origin: vec3<f32>, dir: vec3<f32>, depth: i32, rng: ptr<function, u32>, skip: i32) -> RayResult {
    return trace_ray_base_no_gi(origin, dir, depth, rng, skip);
}

// -----------------------------
// Glass (solid) with fixed exit refraction + distance blur
// -----------------------------
fn shade_glass(
    view_dir: vec3<f32>,    // primary ray direction (camera space), points from camera into scene
               normal:   vec3<f32>,    // geometric surface normal at the entry point
               hit_pos:  vec3<f32>,    // entry point in camera space
               depth:    i32,
               mat_idx:  u32,
               obj:      Object,
               rng:      ptr<function, u32>
) -> vec3<f32> {
    let mat = materials[mat_idx];

    // RR to keep bright rough-glass paths under control
    let rr = russian_roulette(depth, mat.roughnessFactor, rng);
    if (rr == 0.0) { return vec3<f32>(0.0); }

    // --- ENTRY INTERFACE ---
    // Microfacet (rough glass). We refract/reflect against the microfacet normal,
    // but use a physically consistent refraction helper that handles enter/exit.
    let m_enter = sample_ggx(normal, mat.roughnessFactor, rng);

    // Entry reflection
    let refl_dir = normalize(reflect(view_dir, m_enter));
    let ndotR    = abs(dot(normal, refl_dir));
    let eps_R    = mix(0.002, 0.02, 1.0 - ndotR);
    let refl_col = trace_ray(hit_pos + refl_dir * max(eps_R, params.min_ray_offset), refl_dir, depth + 1, rng).color;

    // Entry refraction (air -> glass). If TIR happens on the entry microfacet, fall back to reflection.
    let refr_in = refract_ray(view_dir, m_enter, mat.ior);
    if (length(refr_in) == 0.0) {
        return refl_col * rr;
    }

    // Small step inside to avoid immediately re-hitting the entry face
    let dir_in   = normalize(refr_in);
    let eps_in   = 0.002;
    let origin_in = hit_pos + dir_in * eps_in;

    // --- FIND EXIT OF THE *SAME* OBJECT ---
    var exit_t: f32 = 1e20;
    var exit_n: vec3<f32> = vec3<f32>(0.0, 0.0, 1.0);

    if (obj.is_cube > 0u) {
        let bh = cube_hit(origin_in, dir_in, obj.position, obj.size, obj.orientation, obj.scale);
        exit_t = bh.t; exit_n = bh.n;
    } else if (obj.is_mesh > 0u) {
        let mh = mesh_intersect(origin_in, dir_in, obj);
        exit_t = mh.t; exit_n = mh.n;
    } else {
        // sphere
        let t = sphere_intersect(origin_in, dir_in, obj.position, obj.radius);
        exit_t = t;
        if (t < 1e20) {
            let hp = origin_in + dir_in * t;
            exit_n = normalize(hp - obj.position);
        }
    }

    // If no exit was found (degenerate case), just return the entry reflection
    if (exit_t >= 1e19) {
        return refl_col * rr;
    }

    // Point on the *inner* side of the exit interface
    let hit_exit = origin_in + dir_in * exit_t;

    // --- EXIT INTERFACE ---
    let m_exit = sample_ggx(exit_n, mat.roughnessFactor, rng);

    // Inside (glass) -> outside (air) refraction
    var dir_out = refract_ray(dir_in, m_exit, mat.ior);
    if (length(dir_out) == 0.0) {
        // Internal reflection at the inner face — trace that and mix by Fresnel
        let r_inside = normalize(reflect(dir_in, m_exit));
        let col_bounce = trace_ray(hit_exit + r_inside * 0.003, r_inside, depth + 1, rng).color;

        // Schlick based on entry view angle
        let F0g = mat.f0;
        let cos_theta = abs(dot(-view_dir, normal));
        let fresnel = F0g + (vec3<f32>(1.0) - F0g) * pow(1.0 - cos_theta, 5.0);

        var tint = mat.baseColorFactor.rgb;
        tint = select(vec3<f32>(1.0), tint, max(max(tint.x, tint.y), tint.z) > 0.0);

        return mix(col_bounce * tint, refl_col, fresnel) * rr;
    }

    // --- DISTANCE-DEPENDENT TRANSMISSION BLUR (rough glass) ---
    dir_out = normalize(dir_out);
    let eps_out = 0.002;

    // Peek distance to the next surface *behind* the glass to scale blur amount
    let peek = object_tlas_intersect(hit_exit + dir_out * eps_out, dir_out, -1);
    var dist_behind = peek.t;
    if (peek.idx < 0) { dist_behind = 4.0; }  // "far" background fallback

    // Blur grows with roughness and distance. Tweak these to taste.
    let max_angle = 0.25;  // ≈14°
    let d_scale   = 2.0;   // distance where blur is ~half of max for roughness=1
    let blur_amt  = clamp(mat.roughnessFactor, 0.0, 1.0)
    * clamp(dist_behind / d_scale, 0.0, 1.5);

    // Jitter in a cone around dir_out (single sample; converges with TAA/multiple frames)
    let up_vec = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(dir_out.z) > 0.999);
    let tx = normalize(cross(up_vec, dir_out));
    let ty = cross(dir_out, tx);
    let d   = random_in_unit_disk(rng);
    let cone_angle = max_angle * blur_amt;
    let jitter = (tx * d.x + ty * d.y) * tan(cone_angle);
    let dir_out_jitter = normalize(dir_out + jitter);

    let refr_col = trace_ray(hit_exit + dir_out_jitter * eps_out, dir_out_jitter, depth + 1, rng).color;

    // Transmission tint (optional)
    var tint = mat.baseColorFactor.rgb;
    tint = select(vec3<f32>(1.0), tint, max(max(tint.x, tint.y), tint.z) > 0.0);

    // Fresnel mix (use entry view angle for stable energy split)
    let F0g = mat.f0;
    let cos_theta = abs(dot(-view_dir, normal));
    let fresnel = F0g + (vec3<f32>(1.0) - F0g) * pow(1.0 - cos_theta, 5.0);

    return mix(refr_col * tint, refl_col, fresnel) * rr;
}



fn shade_raster_visible_pixel(id: vec2<u32>, uv: vec2<f32>, rng: ptr<function, u32>) {
    let dims = textureDimensions(color_tex);
    let albedo_sample = textureLoad(gbuf_albedo, vec2<i32>(id), 0);
    let encoded_normal = textureLoad(gbuf_normal, vec2<i32>(id), 0);
    let mat_sample = textureLoad(gbuf_material, vec2<i32>(id), 0);
    let device_depth = textureLoad(depth_tex, vec2<i32>(id)).x;

    var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), 1.0, 1.0);
    var far_world = params.inv_view_proj * clip;
    far_world = far_world / far_world.w;
    let view_dir = normalize(far_world.xyz - params.camera_pos.xyz);

    if (device_depth >= 0.9999 || albedo_sample.a <= 0.0) {
        // Primitive `Object`s are not drawn by the mesh G-buffer pass. In
        // raster/hybrid modes, recover those pixels with one primary ray so
        // built-in cubes/spheres remain visible instead of falling through to
        // a black/background-only frame.
        if (params.num_objects > 0) {
            // Keep this fallback deliberately cheap: it is evaluated for empty
            // G-buffer pixels. Calling the full ray/path shading stack here can
            // recursively cast shadow/GI rays over most of the screen and stall
            // raster startup. A single TLAS hit plus direct material lighting is
            // enough to make primitive objects visible in raster modes.
            let primary = object_tlas_intersect(params.camera_pos.xyz, view_dir, -1);
            if (primary.idx >= 0) {
                let mat = materials[objects[u32(primary.idx)].material_index];
                let base = mat.baseColorFactor.rgb;
                var lit = mat.emissiveFactor * mat.emissiveStrength + base * params.skycolor.rgb * 0.18;
                if (params.dir_light_dir.w > 0.0) {
                    let ldir = normalize(-params.dir_light_dir.xyz);
                    let ndotl = max(dot(primary.n, ldir), 0.0);
                    lit = lit + base * params.dir_light_color.xyz * ndotl * params.dir_light_dir.w;
                }
                let shaded = apply_atmosphere(params.camera_pos.xyz, view_dir, primary.t, lit);
                let cloud_result = composite_clouds(params.camera_pos.xyz, view_dir, primary.t, shaded, id);
                textureStore(cloud_radiance_tex, vec2<i32>(id), vec4<f32>(cloud_result.radiance, 1.0));
                textureStore(cloud_transmittance_tex, vec2<i32>(id), vec4<f32>(cloud_result.transmittance, 0.0, 0.0, 1.0));
                textureStore(color_tex, vec2<i32>(id), vec4<f32>(cloud_result.color, 0.0));
                textureStore(normal_tex, vec2<i32>(id), vec4<f32>(primary.n, primary.t));
                textureStore(gi_noisy, vec2<i32>(id), vec4<f32>(0.0, 0.0, 0.0, 0.0));
                return;
            }
        }

        // Raster-primary and hybrid modes still use the LUT atmosphere path;
        // clouds remain a separate pass composited against final scene depth.
        let sky = apply_atmosphere(params.camera_pos.xyz, view_dir, 1e9, params.skycolor.rgb);
        let cloud_result = composite_clouds(params.camera_pos.xyz, view_dir, 1e9, sky, id);
        textureStore(cloud_radiance_tex, vec2<i32>(id), vec4<f32>(cloud_result.radiance, 1.0));
        textureStore(cloud_transmittance_tex, vec2<i32>(id), vec4<f32>(cloud_result.transmittance, 0.0, 0.0, 1.0));
        textureStore(color_tex, vec2<i32>(id), vec4<f32>(cloud_result.color, -1.0));
        textureStore(normal_tex, vec2<i32>(id), vec4<f32>(0.0, 0.0, 0.0, 1.0));
        textureStore(gi_noisy, vec2<i32>(id), vec4<f32>(0.0, 0.0, 0.0, -1.0));
        return;
    }

    var clip_hit = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), device_depth, 1.0);
    var world_hit = params.inv_view_proj * clip_hit;
    world_hit = world_hit / world_hit.w;
    let hit_pos = world_hit.xyz;
    let linear_depth = length(hit_pos - params.camera_pos.xyz);
    let normal = normalize(encoded_normal.xyz * 2.0 - vec3<f32>(1.0));
    let albedo = albedo_sample.rgb;
    let metallic = f32(mat_sample.r) / 255.0;
    let roughness = max(f32(mat_sample.g) / 255.0, 0.04);
    let light_dir = normalize(-params.dir_light_dir.xyz);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    var visibility = 1.0;
    if (params.renderer_mode == RENDERER_MODE_HYBRID_EFFECTS && params.raytraced_shadows_enabled != 0u && n_dot_l > 0.0) {
        let visible = is_visible(hit_pos + normal * 0.02, hit_pos + light_dir * params.shadow_max_distance, 0xffffffffu, 0xffffffffu);
        visibility = select(0.0, 1.0, visible);
    }
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let half_vec = normalize(light_dir - view_dir);
    let spec_power = max(2.0, (1.0 - roughness) * 128.0);
    let spec = pow(max(dot(normal, half_vec), 0.0), spec_power);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - max(dot(-view_dir, half_vec), 0.0), 5.0);
    var direct = (albedo * (1.0 - metallic) / 3.14159265 + fresnel * spec) * params.dir_light_color.rgb * params.dir_light_dir.w * n_dot_l * visibility;
    var ambient = albedo * params.skycolor.rgb * max(0.03, 1.0 - params.sky_occlusion) * 0.18;
    var gi = vec3<f32>(0.0);
    if (params.renderer_mode == RENDERER_MODE_HYBRID_EFFECTS && gi_params.quality != 3u) {
        gi = sample_diffuse_gi(hit_pos + normal * 0.01, normal, rng);
        ambient += gi * albedo;
    }
    let lit = apply_atmosphere(params.camera_pos.xyz, view_dir, linear_depth, direct + ambient);
    let cloud_result = composite_clouds(params.camera_pos.xyz, view_dir, linear_depth, lit, id);
    textureStore(cloud_radiance_tex, vec2<i32>(id), vec4<f32>(cloud_result.radiance, 1.0));
    textureStore(cloud_transmittance_tex, vec2<i32>(id), vec4<f32>(cloud_result.transmittance, 0.0, 0.0, 1.0));
    textureStore(color_tex, vec2<i32>(id), vec4<f32>(cloud_result.color, 0.0));
    textureStore(normal_tex, vec2<i32>(id), vec4<f32>(normal, linear_depth));
    textureStore(gi_noisy, vec2<i32>(id), vec4<f32>(gi, 0.0));
}

// -----------------------------
// Thin-lens camera sampling (DOF)
// -----------------------------
struct CamRay { ro: vec3<f32>, rd: vec3<f32> };

fn sample_camera_ray(view_dir: vec3<f32>, rng: ptr<function, u32>) -> CamRay {
    if (params.dof_enable == 0u || params.dof_aperture <= 0.0 || params.dof_focus_dist <= 0.0) {
        return CamRay(vec3<f32>(0.0), normalize(view_dir));
    }

    let f = normalize(params.camera_front.xyz);
    let r = normalize(params.camera_right.xyz);
    let u = normalize(params.camera_up.xyz);

    let vdir = normalize(view_dir);
    let denom = max(1e-4, dot(vdir, f));
    let t_focus = params.dof_focus_dist / denom;
    let p_focus = vdir * t_focus;

    let lens_radius = 0.5 * params.dof_aperture;
    let d = random_in_unit_disk(rng) * lens_radius;
    let offset = r * d.x + u * d.y;

    let ro = offset;
    let rd = normalize(p_focus - offset);
    return CamRay(ro, rd);
}

fn heatmap(v: f32) -> vec3<f32> {
    let x = clamp(v, 0.0, 1.0);
    return clamp(vec3<f32>(x * 2.0, 1.0 - abs(x * 2.0 - 1.0), (1.0 - x) * 2.0), vec3<f32>(0.0), vec3<f32>(1.0));
}

// -----------------------------
// Main
// -----------------------------
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(color_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }

    let uv = (vec2<f32>(f32(id.x), f32(id.y)) + 0.5) / vec2<f32>(dims);
    var rng_state: u32 = init_rng(id.xy, u32(params.frame_number));

    if (!uses_path_traced_primary_visibility()) {
        shade_raster_visible_pixel(id.xy, uv, &rng_state);
        return;
    }

    var view_dir: vec3<f32>;
    var hit_pos: vec3<f32>;
    var surf_normal: vec3<f32>;
    var out_normal: vec3<f32>;
    var out_depth: f32;
    var out_obj: i32;
    var gi_pos: vec3<f32>;
    var gi_normal: vec3<f32>;
    var final_col: vec3<f32>;
    var debug_ro = vec3<f32>(0.0);
    var debug_rd = vec3<f32>(0.0);

    if (params.is_fisheye != 0) {
        var uvn = uv * 2.0 - vec2<f32>(1.0);
        uvn.x *= f32(dims.x) / f32(dims.y);
        let r = length(uvn);
        let theta = atan2(r, 1.0) * 2.0;
        let phi = atan2(uvn.y, uvn.x);
        let fwd = normalize(params.camera_front.xyz);
        let rvec = normalize(cross(fwd, params.camera_up.xyz));
        let upv = cross(rvec, fwd);
        view_dir = cos(theta) * fwd + sin(theta) * (cos(phi) * rvec + sin(phi) * upv);

        let cam_ray = sample_camera_ray(view_dir, &rng_state);
        debug_ro = cam_ray.ro; debug_rd = cam_ray.rd;
        let primary = trace_ray(cam_ray.ro, cam_ray.rd, 0, &rng_state);
        hit_pos = cam_ray.ro + cam_ray.rd * primary.depth;
        view_dir = cam_ray.rd;

        surf_normal = primary.normal;
        out_normal = surf_normal;
        out_depth = primary.depth;
        out_obj = primary.obj_idx;
        gi_pos = hit_pos;
        gi_normal = surf_normal;

        if (primary.obj_idx >= 0) {
            let obj = objects[u32(primary.obj_idx)];
            var mat_idx = obj.material_index;
            if (primary.tri_idx != 0xffffffffu) { mat_idx = triangles[primary.tri_idx].material_index; }
            let mat = materials[mat_idx];
            if (obj.is_glass > 0u) {
                final_col = shade_glass(view_dir, surf_normal, hit_pos, 0, mat_idx, obj, &rng_state);            } else {
                    let rough = mat.roughnessFactor;
                    let rr = russian_roulette(1, rough, &rng_state);
                    if (rr == 0.0) { final_col = primary.color; }
                    else if (rough <= 0.02) {
                        let refl_dir = normalize(reflect(view_dir, surf_normal));
                        let res = trace_ray(hit_pos + surf_normal * 0.01, refl_dir, 1, &rng_state);
                        let F0 = mix(vec3<f32>(0.04), mat.baseColorFactor.rgb, mat.metallicFactor);
                        let cos_theta = abs(dot(-view_dir, surf_normal));
                        let fresnel = F0 + (vec3<f32>(1.0) - F0) * pow(1.0 - cos_theta, 5.0);
                        final_col = mix(primary.color, res.color * rr, fresnel);
                    } else if (rough < 0.96) {
                        let h = sample_ggx(surf_normal, rough, &rng_state);
                        let refl_dir = normalize(reflect(view_dir, h));
                        let res = trace_ray(hit_pos + h * 0.01, refl_dir, 1, &rng_state);
                        let F0 = mix(vec3<f32>(0.04), mat.baseColorFactor.rgb, mat.metallicFactor);
                        let cos_theta = abs(dot(-view_dir, surf_normal));
                        let fresnel = F0 + (vec3<f32>(1.0) - F0) * pow(1.0 - cos_theta, 5.0);
                        final_col = mix(primary.color, res.color * rr, fresnel);
                    } else {
                        final_col = primary.color;
                    }
                }
        } else {
            out_depth = 1.0; out_normal = vec3<f32>(0.0); out_obj = -1; gi_normal = vec3<f32>(0.0);
            final_col = primary.color;
        }
    } else {
        var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), 1.0, 1.0);
        var world = params.inv_view_proj * clip; world = world / world.w;
        view_dir = normalize(world.xyz);

        let cam_ray = sample_camera_ray(view_dir, &rng_state);
        debug_ro = cam_ray.ro; debug_rd = cam_ray.rd;
        let primary = trace_ray(cam_ray.ro, cam_ray.rd, 0, &rng_state);
        hit_pos = cam_ray.ro + cam_ray.rd * primary.depth;
        view_dir = cam_ray.rd;

        surf_normal = primary.normal;
        out_depth = primary.depth;
        out_normal = primary.normal;
        out_obj = primary.obj_idx;
        gi_pos = hit_pos;
        gi_normal = surf_normal;

        if (primary.obj_idx >= 0) {
            let obj = objects[u32(primary.obj_idx)];
            var mat_idx = obj.material_index;
            if (primary.tri_idx != 0xffffffffu) { mat_idx = triangles[primary.tri_idx].material_index; }
            let mat = materials[mat_idx];
            if (obj.is_glass > 0u) {
                final_col = shade_glass(view_dir, surf_normal, hit_pos, 0, mat_idx, obj, &rng_state);            } else {
                    let rough = mat.roughnessFactor;
                    let rr = russian_roulette(1, rough, &rng_state);
                    if (rr == 0.0) { final_col = primary.color; }
                    else if (rough <= 0.02) {
                        let refl_dir = normalize(reflect(view_dir, surf_normal));
                        let res = trace_ray(hit_pos + surf_normal * 0.01, refl_dir, 1, &rng_state);
                        let F0 = mix(vec3<f32>(0.04), mat.baseColorFactor.rgb, mat.metallicFactor);
                        let cos_theta = abs(dot(-view_dir, surf_normal));
                        let fresnel = F0 + (vec3<f32>(1.0) - F0) * pow(1.0 - cos_theta, 5.0);
                        final_col = mix(primary.color, res.color * rr, fresnel);
                    } else if (rough < 0.96) {
                        let h = sample_ggx(surf_normal, rough, &rng_state);
                        let refl_dir = normalize(reflect(view_dir, h));
                        let res = trace_ray(hit_pos + h * 0.01, refl_dir, 1, &rng_state);
                        let F0 = mix(vec3<f32>(0.04), mat.baseColorFactor.rgb, mat.metallicFactor);
                        let cos_theta = abs(dot(-view_dir, surf_normal));
                        let fresnel = F0 + (vec3<f32>(1.0) - F0) * pow(1.0 - cos_theta, 5.0);
                        final_col = mix(primary.color, res.color * rr, fresnel);
                    } else {
                        final_col = primary.color;
                    }
                }
        } else {
            out_depth = 1.0; out_normal = vec3<f32>(0.0); out_obj = -1; gi_normal = vec3<f32>(0.0);
            final_col = primary.color;
        }
    }

    var gi = vec3<f32>(0.0);
    if (gi_params.quality != 3u) {
        gi = sample_diffuse_gi(gi_pos + gi_normal * 0.01, gi_normal, &rng_state);
    }

    let cloud_scene_depth = select(out_depth, 1e9, out_obj < 0);
    let cloud_result = composite_clouds(vec3<f32>(0.0), view_dir, cloud_scene_depth, final_col, id.xy);
    textureStore(cloud_radiance_tex, vec2<i32>(id.xy), vec4<f32>(cloud_result.radiance, 1.0));
    textureStore(cloud_transmittance_tex, vec2<i32>(id.xy), vec4<f32>(cloud_result.transmittance, 0.0, 0.0, 1.0));
    var debug_col = vec3<f32>(0.0);
    if (params.rt_debug_view != 0u) {
        let primary_dbg = object_tlas_intersect(debug_ro, debug_rd, -1);
        let ray_cost = f32(primary_dbg.tlas_visits + primary_dbg.blas_visits + primary_dbg.tri_tests);
        if (params.rt_debug_view == 1u) { debug_col = heatmap(ray_cost / 128.0); }
        else if (params.rt_debug_view == 2u) { debug_col = heatmap(f32(primary_dbg.tri_tests) / 64.0); }
        else if (params.rt_debug_view == 3u) { debug_col = heatmap(f32(primary_dbg.tlas_visits + primary_dbg.blas_visits) / 128.0); }
        else if (params.rt_debug_view == 4u) { debug_col = heatmap(f32(primary_dbg.tlas_visits + primary_dbg.blas_visits) / 96.0); }
        else if (params.rt_debug_view == 5u) { debug_col = heatmap(f32(primary_dbg.blas_visits + primary_dbg.tri_tests) / 128.0); }
        else if (params.rt_debug_view == 6u) {
            // Fallback overlay: green = RT/high quality, cyan = SSR/probe/material fallback,
            // amber = raster/contact fallback, blue = ambient/lightmap-style fallback.
            if (out_obj >= 0) {
                let obj = objects[u32(out_obj)];
                let mat = materials[obj.material_index];
                let raster_or_probe = (mat.material_flags0 & 0x6u) != 0u || obj.casts_raytraced_shadow == 0u;
                let rough_probe = mat.roughnessFactor > 0.6;
                if ((mat.material_flags0 & 0x1u) != 0u && !rough_probe) { debug_col = vec3<f32>(0.0, 1.0, 0.15); }
                else if (raster_or_probe || rough_probe) { debug_col = vec3<f32>(0.0, 0.75, 1.0); }
                else if (params.raytraced_shadows_enabled == 0u) { debug_col = vec3<f32>(1.0, 0.65, 0.0); }
                else { debug_col = vec3<f32>(0.15, 0.25, 1.0); }
            } else { debug_col = vec3<f32>(0.02, 0.02, 0.05); }
        }
        textureStore(color_tex, vec2<i32>(id.xy), vec4(debug_col, f32(out_obj)));
        textureStore(depth_tex, vec2<i32>(id.xy), vec4(out_depth, 0.0, 0.0, 1.0));
        textureStore(normal_tex, vec2<i32>(id.xy), vec4(out_normal, out_depth));
        textureStore(gi_noisy, vec2<i32>(id.xy), vec4<f32>(debug_col, f32(out_obj)));
        return;
    }

    let L = cloud_result.color;
    let max_lum = 10.0;
    let lum = dot(L, vec3<f32>(0.299, 0.587, 0.114));
    let final_tone = L * min(1.0, max_lum / (lum + 1e-4));

    textureStore(color_tex,  vec2<i32>(id.xy), vec4(final_tone, f32(out_obj)));
    textureStore(depth_tex,  vec2<i32>(id.xy), vec4(out_depth, 0.0, 0.0, 1.0));
    textureStore(normal_tex, vec2<i32>(id.xy), vec4(out_normal, out_depth));
    textureStore(gi_noisy,   vec2<i32>(id.xy), vec4<f32>(gi, f32(out_obj)));
}
