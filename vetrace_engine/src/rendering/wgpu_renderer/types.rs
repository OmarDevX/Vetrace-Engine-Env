use crate::math::Mat4;
use crate::scene::object::{GpuAtmosphere, MAX_ATMOSPHERES};
use bytemuck::{Pod, Zeroable};
use wgpu::TextureView;

// Higher resolution improves SDFGI quality on larger objects
pub const GI_SDF_RES: u32 = 64;
pub const GI_QUALITY_OFF: u32 = 3;
pub const GI_MODE_SDF: u32 = 0;
pub const GI_MODE_PATH: u32 = 1;

/// Matrix that converts OpenGL NDC to WGPU's coordinate system.
/// Accounts for the different Y orientation and depth range.
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 1.0,
]);

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct GiParams {
    pub quality: u32,
    pub debug_mode: u32,
    pub mode: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct PostFxUniforms {
    pub dof_enabled: u32,
    pub dof_manual: u32,
    pub dof_show_focus: u32,
    pub _dof_pad: u32,
    pub dof_focal_depth: f32,
    pub dof_focal_length: f32,
    pub dof_fstop: f32,
    pub dof_coc: f32,
    pub dof_ndof_start: f32,
    pub dof_ndof_dist: f32,
    pub dof_fdof_start: f32,
    pub dof_fdof_dist: f32,
    pub dof_max_blur: f32,
    pub dof_threshold: f32,
    pub dof_gain: f32,
    pub dof_bias: f32,
    pub dof_fringe: f32,
    pub dof_namount: f32,
    pub dof_samples: u32,
    pub dof_rings: u32,
    pub dof_noise: u32,
    pub dof_vignetting: u32,
    pub dof_autofocus: u32,
    pub dof_depth_blur: u32,
    pub dof_vignout: f32,
    pub dof_vignin: f32,
    pub dof_vignfade: f32,
    pub dof_focus_x: f32,
    pub dof_focus_y: f32,
    pub dof_db_size: f32,
    pub dof_feather: f32,
    pub dof_pentagon: u32,
    pub _dof_pad1: u32,
    pub z_near: f32,
    pub z_far: f32,
    pub bloom_enabled: u32,
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub bloom_spread: f32,
    pub bloom_iterations: u32,
    pub exposure: f32,
    pub auto_exposure: u32,
    pub sky_occlusion: f32,
    pub fog_density: f32,
    pub fog_color_r: f32,
    pub fog_color_g: f32,
    pub fog_color_b: f32,
    pub history_clamp_k: f32,
    pub temporal_blend: f32,
    pub gi_temporal_blend: f32,
    pub _pad0: u32,
    pub _pad1: u32,
    pub _pad2: u32,
    pub _pad3: u32,
}

impl Default for PostFxUniforms {
    fn default() -> Self {
        Self {
            dof_enabled: 0,
            dof_manual: 0,
            dof_show_focus: 0,
            _dof_pad: 0,
            dof_focal_depth: 0.0,
            dof_focal_length: 0.0,
            dof_fstop: 0.0,
            dof_coc: 0.0,
            dof_ndof_start: 0.0,
            dof_ndof_dist: 0.0,
            dof_fdof_start: 0.0,
            dof_fdof_dist: 0.0,
            dof_max_blur: 1.0,
            dof_threshold: 0.7,
            dof_gain: 100.0,
            dof_bias: 0.5,
            dof_fringe: 0.7,
            dof_namount: 0.0001,
            dof_samples: 3,
            dof_rings: 3,
            dof_noise: 1,
            dof_vignetting: 0,
            dof_autofocus: 0,
            dof_depth_blur: 0,
            dof_vignout: 1.3,
            dof_vignin: 0.0,
            dof_vignfade: 22.0,
            dof_focus_x: 0.5,
            dof_focus_y: 0.5,
            dof_db_size: 1.25,
            dof_feather: 0.4,
            dof_pentagon: 0,
            _dof_pad1: 0,
            z_near: 0.1,
            z_far: 1000.0,
            bloom_enabled: 0,
            bloom_threshold: 1.0,
            bloom_intensity: 0.0,
            bloom_spread: 2.0,
            bloom_iterations: 5,
            exposure: 1.0,
            auto_exposure: 0,
            sky_occlusion: 0.0,
            fog_density: 0.0,
            fog_color_r: 1.0,
            fog_color_g: 1.0,
            fog_color_b: 1.0,
            history_clamp_k: 1.5,
            // Higher values accumulate more history in the temporal filter
            temporal_blend: 1.0,
            gi_temporal_blend: 0.1,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct ShaderParams {
    pub camera_pos: [f32; 4],
    pub camera_front: [f32; 4],
    pub camera_up: [f32; 4],
    pub camera_right: [f32; 4],
    pub prev_camera_pos: [f32; 4],
    pub fov: f32,
    pub num_objects: i32,
    pub is_fisheye: i32,
    pub _pad0: i32,
    pub skycolor: [f32; 4],
    pub taa_jitter: [f32; 2],
    pub current_time: f32,
    pub frame_number: i32,
    pub selected_index: i32,
    pub max_bounces: i32,
    pub light_samples: i32,
    pub dir_shadow_samples: i32,
    pub inv_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
    pub dir_light_dir: [f32; 4],
    pub dir_light_color: [f32; 4],
    pub sky_occlusion: f32,
    pub total_triangles: u32,
    pub total_bvh_nodes: u32,
    pub total_tri_bvh_nodes: u32,
    pub dof_aperture: f32,
    pub dof_focus_dist: f32,
    pub dof_enable: u32,
    pub _pad_dof: u32,
    pub atmosphere: u32,
    pub atmo_count: u32,
    pub _pad_atmos: [u32; 2],
    pub atmos: [GpuAtmosphere; MAX_ATMOSPHERES],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct BlitParams {
    pub camera_pos: [f32; 4],
    pub prev_camera_pos: [f32; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
    pub taa_jitter: [f32; 2],
    pub prev_taa_jitter: [f32; 2],
    pub tex_size: [f32; 2],
    pub sharpness: f32,
    pub selected_index: i32,
    pub _pad0: [i32; 2],
    pub _pad1: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct LightListHeader {
    pub count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct LightUniform {
    pub dir: [f32; 2],
    pub _pad: [f32; 2],
    pub color: [f32; 3],
    pub intensity: f32,
}

pub struct SpriteRenderData {
    pub vertices: [[f32; 5]; 6],
    pub texture: std::sync::Arc<TextureView>,
    pub double_sided: bool,
}

pub struct PbrRenderData {
    pub mesh: crate::gpu::MeshHandle,
    pub material: crate::materials::PbrMaterial,
    pub mvp: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub joint_mats: Option<Vec<[[f32; 4]; 4]>>,
}
