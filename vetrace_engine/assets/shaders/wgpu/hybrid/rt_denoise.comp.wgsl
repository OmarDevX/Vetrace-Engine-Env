// Full screen temporal denoiser for the ray traced color buffer

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
    _pad_shadow_mode: u32,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    sky_occlusion: f32,
    total_triangles: u32,
    total_bvh_nodes: u32,
    total_tri_bvh_nodes: u32,
};

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

@group(0) @binding(0) var noisy_tex: texture_2d<f32>;
@group(0) @binding(1) var history_tex: texture_2d<f32>;
@group(0) @binding(2) var accum_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var depth_tex: texture_2d<f32>;
@group(0) @binding(4) var normal_tex: texture_2d<f32>;
@group(0) @binding(5) var motion_tex: texture_storage_2d<rg16float, write>;
@group(0) @binding(6) var variance_tex: texture_storage_2d<r32float, read_write>;
@group(0) @binding(7) var depth_history_tex: texture_2d<f32>;
@group(0) @binding(8) var normal_history_tex: texture_2d<f32>;
@group(0) @binding(9) var<uniform> params: Params;
@group(0) @binding(10) var<uniform> postfx: PostFxUniforms;
@group(0) @binding(11) var<storage, read> objects: array<Object>;

fn get_world_pos(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    // Reconstruct the world position from screen UV and linear depth
    var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), 1.0, 1.0);
    var wpos = params.inv_view_proj * clip;
    wpos = wpos / wpos.w;
    let dir = normalize(wpos.xyz - params.camera_pos.xyz);
    return params.camera_pos.xyz + dir * depth;
}

@compute @workgroup_size(16,16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(accum_tex);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }
    let coord = vec2<i32>(id.xy);
    let noisy_sample = textureLoad(noisy_tex, coord, 0);
    let current = noisy_sample.rgb;
    // Convert the object ID stored in the alpha channel to a precise integer
    // before performing comparisons. Directly casting the float can truncate
    // values due to precision issues in the Rgba16Float buffer, which caused
    // only object ID 2 to pass the equality check and receive history reuse.
    // Round the object ID stored in the alpha channel to avoid precision
    // issues with the 16-bit render target. Without rounding, large IDs could
    // compare incorrectly and disable temporal accumulation entirely.
    let obj = i32(round(noisy_sample.w));
    let current_normal = textureLoad(normal_tex, coord, 0).xyz;

    let uv = (vec2<f32>(f32(id.x), f32(id.y)) + 0.5) / vec2<f32>(dims);
    let depth = textureLoad(depth_tex, coord, 0).x;

    var prev_color = current;
    var motion = vec2<f32>(0.0);
    var use_history = false;
    if (postfx.dof_enabled != 0) {
        // Reuse same-pixel history for DOF to accumulate random lens
        // samples, but reject history if the camera moved too much to
        // avoid ghosting when the view changes.
        if (params.frame_number > 0) {
            let world = get_world_pos(uv, depth);
            let cam_delta = params.camera_pos.xyz - params.prev_camera_pos.xyz;
            let prev_pos = params.prev_view_proj * vec4<f32>(world + cam_delta, 1.0);
            let prev_ndc = prev_pos.xy / prev_pos.w;
            let prev_uv = prev_ndc * vec2<f32>(0.5, 0.5) + vec2<f32>(0.5, 0.5);
            motion = prev_uv - uv;
            let prev_sample = textureLoad(history_tex, coord, 0);
            prev_color = prev_sample.rgb;
            // Only reuse history if the pixel stayed within ~1 screen pixel.
            let motion_px = motion * vec2<f32>(dims);
            if (length(motion_px) < 1.0) {
                use_history = true;
            }
        }
    } else if (params.frame_number > 0) {
        let world = get_world_pos(uv, depth);
        let cam_delta = params.camera_pos.xyz - params.prev_camera_pos.xyz;
        let prev_pos = params.prev_view_proj * vec4<f32>(world + cam_delta, 1.0);
        let prev_ndc = prev_pos.xy / prev_pos.w;
        let prev_uv = prev_ndc * vec2<f32>(0.5, 0.5) + vec2<f32>(0.5, 0.5);
        motion = prev_uv - uv;
        if (all(prev_uv >= vec2<f32>(0.0)) && all(prev_uv < vec2<f32>(1.0))) {
            let prev_coord = vec2<i32>(prev_uv * vec2<f32>(dims));
            let prev_sample = textureLoad(history_tex, prev_coord, 0);
            let prev_depth = textureLoad(depth_history_tex, prev_coord, 0).x;
            let prev_normal = textureLoad(normal_history_tex, prev_coord, 0).xyz;
            let prev_obj = i32(round(prev_sample.w));
            // Use a slightly more permissive normal threshold to account for
            // quantization noise in the normal buffer.
            if (abs(prev_depth - depth) < postfx.history_clamp_k && prev_obj == obj && dot(prev_normal, current_normal) > 0.8) {
                prev_color = prev_sample.rgb;
                use_history = true;
            }
        }
    }
    textureStore(motion_tex, coord, vec4<f32>(motion, 0.0, 0.0));
    var alpha: f32;
    if (use_history) {
        // `temporal_blend` is exposed to users as an accumulation strength
        // slider where larger values should keep more history. Convert it to
        // the current-frame weight used by mix(prev, current, alpha).
        alpha = 1.0 / (max(postfx.temporal_blend, 0.0) + 1.0);
    } else {
        alpha = 1.0;
    }
    let blended = mix(prev_color, current, alpha);
    textureStore(accum_tex, coord, vec4<f32>(blended, f32(obj)));
}
