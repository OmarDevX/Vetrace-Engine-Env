use crate::{
    components::components::{CameraAttachment, PostProcessing, VolumetricFog},
    engine::engine::Engine,
    Behaviour,
};

#[cfg(feature = "wgpu")]
use crate::rendering::wgpu_renderer::PostFxUniforms;

#[cfg(not(feature = "wgpu"))]
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
}

#[cfg(not(feature = "wgpu"))]
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
            temporal_blend: 1.0,
            gi_temporal_blend: 0.1,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        }
    }
}

pub struct PostProcessBehaviour;

impl Behaviour for PostProcessBehaviour {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut uniforms = PostFxUniforms::default();
        let camera_entities: Vec<_> = engine
            .world
            .query::<CameraAttachment>()
            .into_iter()
            .map(|(e, _)| e)
            .collect();
        for entity in camera_entities {
            if let Some(pp) = engine.world.get_mut::<PostProcessing>(entity) {
                uniforms.dof_enabled = 0;
                uniforms.dof_manual = 0;
                uniforms.dof_show_focus = 0;
                uniforms.dof_focal_depth = 0.0;
                uniforms.dof_focal_length = 0.0;
                uniforms.dof_fstop = 0.0;
                uniforms.dof_coc = 0.0;
                uniforms.dof_ndof_start = 0.0;
                uniforms.dof_ndof_dist = 0.0;
                uniforms.dof_fdof_start = 0.0;
                uniforms.dof_fdof_dist = 0.0;
                uniforms.dof_max_blur = 1.0;
                uniforms.dof_threshold = 0.0;
                uniforms.dof_gain = 0.0;
                uniforms.dof_bias = 0.0;
                uniforms.dof_fringe = 0.0;
                uniforms.dof_namount = 0.0;
                uniforms.dof_samples = 0;
                uniforms.dof_rings = 0;
                uniforms.dof_noise = 1;
                uniforms.dof_vignetting = 0;
                uniforms.dof_autofocus = 0;
                uniforms.dof_depth_blur = 0;
                uniforms.dof_vignout = 0.0;
                uniforms.dof_vignin = 0.0;
                uniforms.dof_vignfade = 0.0;
                uniforms.dof_focus_x = 0.5;
                uniforms.dof_focus_y = 0.5;
                uniforms.dof_db_size = 1.0;
                uniforms.dof_feather = 0.0;
                uniforms.dof_pentagon = 0;
                let cam_pos = engine.active_camera_info().position;
                uniforms.z_near = engine.scene.camera_near_plane(cam_pos);
                uniforms.z_far = 1000.0;
                if let Some(d) = &pp.dof {
                    uniforms.dof_enabled = 1;
                    uniforms.dof_manual = if d.manual { 1 } else { 0 };
                    uniforms.dof_show_focus = if d.show_focus { 1 } else { 0 };
                    uniforms.dof_focal_depth = d.focal_depth;
                    uniforms.dof_focal_length = d.focal_length;
                    uniforms.dof_fstop = d.fstop;
                    uniforms.dof_coc = d.coc;
                    uniforms.dof_ndof_start = d.ndof_start;
                    uniforms.dof_ndof_dist = d.ndof_dist;
                    uniforms.dof_fdof_start = d.fdof_start;
                    uniforms.dof_fdof_dist = d.fdof_dist;
                    uniforms.dof_max_blur = d.max_blur;
                    uniforms.dof_threshold = d.threshold;
                    uniforms.dof_gain = d.gain;
                    uniforms.dof_bias = d.bias;
                    uniforms.dof_fringe = d.fringe;
                    uniforms.dof_namount = d.namount;
                    uniforms.dof_samples = d.samples;
                    uniforms.dof_rings = d.rings;
                    uniforms.dof_noise = if d.noise { 1 } else { 0 };
                    uniforms.dof_vignetting = if d.vignetting { 1 } else { 0 };
                    uniforms.dof_autofocus = if d.autofocus { 1 } else { 0 };
                    uniforms.dof_depth_blur = if d.depth_blur { 1 } else { 0 };
                    uniforms.dof_vignout = d.vign_out;
                    uniforms.dof_vignin = d.vign_in;
                    uniforms.dof_vignfade = d.vign_fade;
                    uniforms.dof_focus_x = d.focus.x;
                    uniforms.dof_focus_y = d.focus.y;
                    uniforms.dof_db_size = d.db_size;
                    uniforms.dof_feather = d.feather;
                    uniforms.dof_pentagon = if d.pentagon { 1 } else { 0 };
                }
                if let Some(b) = &pp.bloom {
                    uniforms.bloom_enabled = 1;
                    uniforms.bloom_threshold = b.threshold;
                    uniforms.bloom_intensity = b.intensity;
                    uniforms.bloom_spread = b.spread;
                    uniforms.bloom_iterations = b.iterations as u32;
                }
                uniforms.exposure = pp.exposure;
                uniforms.auto_exposure = if pp.auto_exposure { 1 } else { 0 };
                uniforms.sky_occlusion = 0.0;
                uniforms.history_clamp_k = pp.history_clamp_k;
                uniforms.temporal_blend = pp.temporal_blend;
                uniforms.gi_temporal_blend = pp.gi_temporal_blend;
            }
            if let Some(fog) = engine.world.get::<VolumetricFog>(entity) {
                uniforms.fog_density = fog.density;
                uniforms.fog_color_r = fog.color[0];
                uniforms.fog_color_g = fog.color[1];
                uniforms.fog_color_b = fog.color[2];
            }
            break;
        }
        #[cfg(feature = "wgpu")]
        engine.renderer.set_post_fx_uniforms(uniforms);
    }
}