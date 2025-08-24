// Hybrid GI denoiser

struct Params {
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_up: vec4<f32>,
    camera_right: vec4<f32>,
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
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    sky_occlusion: f32,
    total_triangles: u32,
    total_bvh_nodes: u32,
    total_tri_bvh_nodes: u32,
};

struct PostFxUniforms {
    dof_enabled: u32,
    dof_manual: u32,
    dof_show_focus: u32,
    _dof_pad: u32,
    dof_focal_depth: f32,
    dof_focal_length: f32,
    dof_fstop: f32,
    dof_coc: f32,
    dof_ndof_start: f32,
    dof_ndof_dist: f32,
    dof_fdof_start: f32,
    dof_fdof_dist: f32,
    dof_max_blur: f32,
    dof_threshold: f32,
    dof_gain: f32,
    dof_bias: f32,
    dof_fringe: f32,
    dof_namount: f32,
    dof_samples: u32,
    dof_rings: u32,
    dof_noise: u32,
    dof_vignetting: u32,
    dof_autofocus: u32,
    dof_depth_blur: u32,
    dof_vignout: f32,
    dof_vignin: f32,
    dof_vignfade: f32,
    dof_focus_x: f32,
    dof_focus_y: f32,
    dof_db_size: f32,
    dof_feather: f32,
    dof_pentagon: u32,
    _dof_pad1: u32,
    z_near: f32,
    z_far: f32,
    bloom_enabled: u32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    bloom_spread: f32,
    bloom_iterations: u32,
    exposure: f32,
    auto_exposure: u32,
    sky_occlusion: f32,
    fog_density: f32,
    fog_color_r: f32,
    fog_color_g: f32,
    fog_color_b: f32,
    history_clamp_k: f32,
    temporal_blend: f32,
    gi_temporal_blend: f32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
};

@group(0) @binding(4) var<uniform> params: Params;
@group(0) @binding(6) var depth_tex: texture_2d<f32>;
@group(0) @binding(10) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(13) var sdf_tex: texture_3d<f32>;
@group(0) @binding(14) var sdf_sampler: sampler;
@group(0) @binding(15) var gi_history: texture_2d<f32>;
@group(0) @binding(16) var gi_noisy_tex: texture_2d<f32>;
@group(0) @binding(17) var gi_buffer: texture_storage_2d<rgba16float, write>;
@group(0) @binding(18) var<uniform> postfx: PostFxUniforms;

const SDF_MIN: vec3<f32> = vec3<f32>(-10.0, -10.0, -10.0);
const SDF_SIZE: vec3<f32> = vec3<f32>(20.0, 20.0, 20.0);

fn world_to_sdf(p: vec3<f32>) -> vec3<f32> {
    return (p - SDF_MIN) / SDF_SIZE;
}

fn get_world_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), 1.0, 1.0);
    var w = params.inv_view_proj * clip;
    w = w / w.w;
    let dir = normalize(w.xyz - params.camera_pos.xyz);
    return params.camera_pos.xyz + dir * depth;
}

fn reproject_gi(world_pos: vec3<f32>) -> vec3<f32> {
    // Skip reprojection on the first frame or when using fisheye
    if (params.frame_number == 0 || params.is_fisheye != 0) {
        return vec3<f32>(0.0);
    }
    let prev_pos = params.prev_view_proj * vec4<f32>(world_pos, 1.0);
    let ndc = prev_pos.xy / prev_pos.w;
    // Map from NDC [-1,1] to texture UV [0,1]
    let uv = ndc * vec2<f32>(0.5, 0.5) + vec2<f32>(0.5, 0.5);
    let dist = textureSampleLevel(sdf_tex, sdf_sampler, world_to_sdf(world_pos), 0.0).x;
    if (abs(dist) > 0.2) { return vec3<f32>(0.0); }
    return textureSampleLevel(gi_history, sdf_sampler, uv, 0.0).rgb;
}

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(gi_buffer);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let uv = (vec2<f32>(f32(id.x), f32(id.y)) + 0.5) / vec2<f32>(dims);
    let depth = textureLoad(depth_tex, vec2<i32>(id.xy), 0).x;
    let world = get_world_pos(uv, depth);

    var sum = vec3<f32>(0.0);
    var wsum = 0.0;
    let pixel = 1.0 / vec2<f32>(dims);
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let off = vec2<f32>(f32(x), f32(y)) * pixel;
            let suv = uv + off;
            let d = textureLoad(depth_tex, vec2<i32>(suv * vec2<f32>(dims)), 0).x;
            let spos = get_world_pos(suv, d);
            let dist = textureSampleLevel(sdf_tex, sdf_sampler, world_to_sdf(spos), 0.0).x;
            let w = exp(-dist * dist * 50.0);
            sum += textureLoad(gi_noisy_tex, vec2<i32>(suv * vec2<f32>(dims)), 0).xyz * w;
            wsum += w;
        }
    }
    let blurred = sum / wsum;
    var reproj = reproject_gi(world);
    // Clamp the reprojected GI to the neighborhood to suppress large history
    // outliers before blending.
    let clamp_range = vec3<f32>(postfx.history_clamp_k);
    reproj = clamp(reproj, blurred - clamp_range, blurred + clamp_range);
    let gi = mix(blurred, reproj, clamp(postfx.gi_temporal_blend, 0.0, 1.0));
    textureStore(gi_buffer, vec2<i32>(id.xy), vec4<f32>(gi, 1.0));
}