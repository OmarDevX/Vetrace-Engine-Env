use crate::math::Mat4;
use crate::scene::object::{GpuAtmosphere, MAX_ATMOSPHERES};
use bytemuck::{Pod, Zeroable};
use wgpu::TextureView;

// Higher resolution improves SDFGI quality on larger objects
pub const GI_SDF_RES: u32 = 64;
pub const GI_QUALITY_OFF: u32 = 3;
/// Disable indirect diffuse lighting; only direct/raster lighting contributes.
pub const GI_MODE_OFF: u32 = 0;
/// Use authored baked lightmap data for baseline raster indirect lighting.
/// This is a static, low-cost production mode for RasterGame/HybridEffects.
pub const GI_MODE_BAKED_LIGHTMAP: u32 = 1;
/// Use authored/interpolated light probes for baseline raster indirect lighting.
/// This is the scalable default for performance profiles such as Indoor60FPS.
pub const GI_MODE_LIGHT_PROBES: u32 = 2;
/// Use signed-distance-field GI cache/cone sampling for scalable dynamic GI.
/// This is allowed in RasterGame and HybridEffects when the profile budget permits.
pub const GI_MODE_SDFGI: u32 = 3;
/// Use one diffuse ray-traced GI bounce as a HybridEffects-only additive pass.
/// RasterGame must not dispatch this; path-traced primary modes use path tracing instead.
pub const GI_MODE_RTGI_ONE_BOUNCE: u32 = 4;
/// Use path-traced indirect lighting for path-traced primary visibility modes only.
/// RasterGame/HybridEffects requests are clamped to cheaper baked/probe/SDFGI modes.
pub const GI_MODE_PATH_TRACED_PREVIEW: u32 = 5;
// Back-compat aliases for older call sites.
pub const GI_MODE_SDF: u32 = GI_MODE_SDFGI;
pub const GI_MODE_PATH: u32 = GI_MODE_PATH_TRACED_PREVIEW;
// GI resolve shader ABI constants; these intentionally match GiMethod::{Off, BakedLightmap, LightProbes, SDFGI, RTGIOneBounce}.
pub const GI_RESOLVE_METHOD_OFF: u32 = GI_MODE_OFF;
pub const GI_RESOLVE_METHOD_BAKED_LIGHTMAP: u32 = GI_MODE_BAKED_LIGHTMAP;
pub const GI_RESOLVE_METHOD_LIGHT_PROBES: u32 = GI_MODE_LIGHT_PROBES;
pub const GI_RESOLVE_METHOD_SDFGI: u32 = GI_MODE_SDFGI;
pub const GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE: u32 = GI_MODE_RTGI_ONE_BOUNCE;
pub const AO_METHOD_OFF: u32 = 0;
pub const AO_METHOD_SSAO: u32 = 1;
pub const AO_METHOD_GTAO: u32 = 2;
pub const AO_METHOD_RTAO: u32 = 3;

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
pub struct GiResolveParams {
    pub selected_method: u32,
    pub frame_number: u32,
    pub debug_flags: u32,
    pub _pad0: u32,
    pub temporal_blend: f32,
    pub baked_blend: f32,
    pub probe_blend: f32,
    pub sdfgi_blend: f32,
    pub rtgi_blend: f32,
    pub probe_count: u32,
    pub gi_resource_flags: u32,
    pub _pad1: [u32; 2],
    pub sdfgi_origin: [f32; 4],
    pub sdfgi_extent_voxel: [f32; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct GpuLightProbeData {
    pub position_radius: [f32; 4],
    /// x = visibility multiplier, y = artist/importance weight, z/w reserved.
    pub weight_visibility: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct GpuLightProbeSh {
    /// Nine RGB SH/irradiance coefficients per probe, padded to vec4 for WGSL layout.
    pub coeffs: [[f32; 4]; 9],
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
    pub fog_base_height: f32,
    pub fog_height_falloff: f32,
    pub fog_max_opacity: f32,
    pub fog_inscatter_r: f32,
    pub fog_inscatter_g: f32,
    pub fog_inscatter_b: f32,
    pub history_clamp_k: f32,
    pub temporal_blend: f32,
    pub gi_temporal_blend: f32,
    pub shadow_history_weight: f32,
    pub reflection_history_weight: f32,
    pub cloud_history_weight: f32,
    pub denoise_mode: u32,
    pub denoise_debug_view: u32,
    pub _pad0: u32,
    pub _pad1: u32,
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
            fog_base_height: 0.0,
            fog_height_falloff: 0.0,
            fog_max_opacity: 1.0,
            fog_inscatter_r: 1.0,
            fog_inscatter_g: 1.0,
            fog_inscatter_b: 1.0,
            history_clamp_k: 1.5,
            // Higher values accumulate more history in the temporal filter
            temporal_blend: 1.0,
            gi_temporal_blend: 0.1,
            shadow_history_weight: 0.92,
            reflection_history_weight: 0.82,
            cloud_history_weight: 0.90,
            denoise_mode: 0,
            denoise_debug_view: 0,
            _pad0: 0,
            _pad1: 0,
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
    pub shadow_mode: u32,
    pub raytraced_shadows_enabled: u32,
    pub shadow_quality: u32,
    pub max_shadow_rays: u32,
    pub emissive_shadow_samples: u32,
    pub directional_shadow_samples: u32,
    pub cloud_object_shadows_enabled: u32,
    pub max_rt_shadow_distance: f32,
    pub rt_shadow_ray_t_max: f32,
    pub min_soft_shadow_radius: f32,
    pub raytraced_reflections_enabled: u32,
    /// WGSL aligns the following `mat4x4<f32>` field to 16 bytes.
    pub _pad_reflections: u32,
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
    pub cloud_count: u32,
    pub atmosphere_mode: u32,
    pub atmosphere_sun_controls: [f32; 4],
    pub cloud_history_weight: f32,
    pub cloud_sample_count: u32,
    pub cloud_temporal_quality: u32,
    pub cloud_shadow_mode: u32,
    pub renderer_mode: u32,
    pub rt_debug_view: u32,
    pub rt_debug_counters: u32,
    pub max_traversal_steps: u32,
    pub max_transparent_surfaces: u32,
    pub shadow_max_distance: f32,
    pub reflection_max_distance: f32,
    pub gi_max_distance: f32,
    pub min_ray_offset: f32,
    /// Host-side padding for the WGSL uniform layout around `_pad_atmos: vec3<u32>`.
    ///
    /// WGSL inserts 12 bytes of implicit padding before the vec3 and 4 bytes
    /// after it so the following `atmos` array starts on a 16-byte boundary.
    /// Rust `repr(C)` does not insert those implicit uniform-layout gaps, so
    /// this one host padding field covers all 28 bytes:
    ///
    /// - 12 bytes before WGSL `_pad_atmos`
    /// - 12 bytes for WGSL `_pad_atmos: vec3<u32>`
    /// - 4 bytes before `atmos`
    pub _pad_atmos: [u32; 7],
    pub atmos: [GpuAtmosphere; MAX_ATMOSPHERES],
}

const _: [(); 1776] = [(); std::mem::size_of::<ShaderParams>()];

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
pub struct HybridRtEffectParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub dir_light_dir: [f32; 4],
    pub dir_light_color: [f32; 4],
    pub enabled: u32,
    pub mode: u32,
    pub gi_mode: u32,
    pub rtao_sample_count: u32,
    pub rtao_radius_bits: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct SsrParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_proj: [[f32; 4]; 4],
    pub prev_view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub tex_size: [f32; 2],
    pub max_distance: f32,
    pub thickness: f32,
    pub temporal_blend: f32,
    pub roughness_cutoff: f32,
    pub confidence_threshold: f32,
    pub stride: f32,
    pub max_steps: u32,
    pub frame_number: u32,
    pub enabled: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct HybridCompositeParams {
    pub temporal_blend: f32,
    pub rt_gi_enabled: u32,
    pub rt_reflections_enabled: u32,
    pub ssr_enabled: u32,
    pub rt_shadows_enabled: u32,
    pub rt_transparency_enabled: u32,
    pub atmosphere_enabled: u32,
    pub clouds_enabled: u32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
pub struct AmbientOcclusionParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub tex_size: [f32; 2],
    pub radius: f32,
    pub intensity: f32,
    pub method: u32,
    pub frame_number: u32,
    pub temporal_enabled: u32,
    pub _pad: u32,
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

#[cfg(test)]
mod layout_tests {
    use super::ShaderParams;
    use crate::scene::{
        bvh::GpuBvhNode,
        object::{GpuCustomMaterial, GpuMaterial, GpuObject, GpuTriangle},
        tri_bvh::GpuTriBvhNode,
    };

    #[test]
    fn gpu_struct_sizes_match_wgsl_layouts() {
        assert_eq!(std::mem::size_of::<ShaderParams>(), 1776);
        assert_eq!(std::mem::size_of::<GpuObject>(), 144);
        assert_eq!(std::mem::size_of::<GpuTriangle>(), 128);
        assert_eq!(std::mem::size_of::<GpuMaterial>(), 96);
        assert_eq!(std::mem::size_of::<GpuCustomMaterial>(), 144);
        assert_eq!(std::mem::size_of::<GpuBvhNode>(), 48);
        assert_eq!(std::mem::size_of::<GpuTriBvhNode>(), 48);
    }

    const SHADER_PARAMS_PREFIX: &[&str] = &[
        "camera_pos",
        "camera_front",
        "camera_up",
        "camera_right",
        "prev_camera_pos",
        "fov",
        "num_objects",
        "is_fisheye",
        "_pad0",
        "skycolor",
        "taa_jitter",
        "current_time",
        "frame_number",
        "selected_index",
        "max_bounces",
        "light_samples",
        "dir_shadow_samples",
        "shadow_mode",
        "raytraced_shadows_enabled",
        "shadow_quality",
        "max_shadow_rays",
        "emissive_shadow_samples",
        "directional_shadow_samples",
        "cloud_object_shadows_enabled",
        "max_rt_shadow_distance",
        "rt_shadow_ray_t_max",
        "min_soft_shadow_radius",
        "raytraced_reflections_enabled",
        "_pad_reflections",
        "inv_view_proj",
        "prev_view_proj",
        "dir_light_dir",
        "dir_light_color",
        "sky_occlusion",
        "total_triangles",
        "total_bvh_nodes",
        "total_tri_bvh_nodes",
        "dof_aperture",
        "dof_focus_dist",
        "dof_enable",
        "_pad_dof",
        "atmosphere",
        "atmo_count",
        "cloud_count",
        "atmosphere_mode",
        "atmosphere_sun_controls",
        "cloud_history_weight",
        "cloud_sample_count",
        "cloud_temporal_quality",
        "cloud_shadow_mode",
        "renderer_mode",
        "rt_debug_view",
        "rt_debug_counters",
        "max_traversal_steps",
        "max_transparent_surfaces",
        "shadow_max_distance",
        "reflection_max_distance",
        "gi_max_distance",
        "min_ray_offset",
        "_pad_atmos",
        "atmos",
    ];

    const MATERIAL_PARAMS_FIELDS: &[&str] = &[
        "baseColorFactor",
        "emissiveFactor",
        "emissiveStrength",
        "metallicFactor",
        "roughnessFactor",
        "ior",
        "baseColorTex",
        "f0",
        "has_custom_material",
        "custom_material_id",
        "material_flags0",
        "material_flags1",
        "material_flags2",
        "material_flags3",
        "material_flags4",
        "material_flags5",
        "material_flags6",
    ];

    #[test]
    fn wgsl_params_prefixes_match_shader_params() {
        for (name, source) in WGSL_PARAMS_SHADERS {
            let fields = wgsl_struct_fields(source, "Params");
            assert!(
                SHADER_PARAMS_PREFIX.starts_with(&fields),
                "{name} Params does not match ShaderParams prefix; fields were {fields:?}"
            );
        }
    }

    #[test]
    fn wgsl_material_params_match_gpu_material_stride_fields() {
        for (name, source) in WGSL_MATERIAL_SHADERS {
            let fields = wgsl_struct_fields(source, "MaterialParams");
            assert_eq!(
                fields, MATERIAL_PARAMS_FIELDS,
                "{name} MaterialParams must keep seven trailing u32 flag fields so WGSL stride matches 96-byte GpuMaterial"
            );
            assert!(
                !source.contains("mat._pad2"),
                "{name} still reads the old vec3 padding field"
            );
        }
    }

    const WGSL_PARAMS_SHADERS: &[(&str, &str)] = &[
        (
            "pathtrace.comp.wgsl",
            concat!(
                include_str!("../../../assets/shaders/wgpu/hybrid/pbr_lighting.wgsl"),
                "\n",
                include_str!("../../../assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl"),
            ),
        ),
        (
            "denoise.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/denoise.comp.wgsl"),
        ),
        (
            "rt_denoise.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/rt_denoise.comp.wgsl"),
        ),
        (
            "sdfgi_prepass.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/sdfgi_prepass.comp.wgsl"),
        ),
        (
            "sdfgi_inject.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/sdfgi_inject.comp.wgsl"),
        ),
        (
            "transmittance_lut.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/atmosphere/transmittance_lut.comp.wgsl"),
        ),
        (
            "sky_view_lut.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/atmosphere/sky_view_lut.comp.wgsl"),
        ),
        (
            "multi_scattering_lut.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/atmosphere/multi_scattering_lut.comp.wgsl"),
        ),
        (
            "aerial_perspective_lut.comp.wgsl",
            include_str!(
                "../../../assets/shaders/wgpu/atmosphere/aerial_perspective_lut.comp.wgsl"
            ),
        ),
    ];

    const WGSL_MATERIAL_SHADERS: &[(&str, &str)] = &[(
        "pathtrace.comp.wgsl",
        concat!(
            include_str!("../../../assets/shaders/wgpu/hybrid/pbr_lighting.wgsl"),
            "\n",
            include_str!("../../../assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl"),
        ),
    )];

    #[test]
    fn standalone_hybrid_wgsl_modules_parse() {
        for (name, source) in STANDALONE_HYBRID_WGSL {
            naga::front::wgsl::parse_str(source)
                .unwrap_or_else(|err| panic!("{name} failed standalone WGSL parsing: {err}"));
        }
    }

    #[test]
    fn hybrid_bind_group_layouts_match_shader_bindings() {
        for (name, source, expected) in HYBRID_BINDING_LAYOUTS {
            let mut bindings = wgsl_bindings(source);
            bindings.sort_unstable();
            assert_eq!(bindings, *expected, "{name} bind group bindings drifted");
        }
    }

    #[test]
    fn hybrid_debug_view_smoke_paths_are_present() {
        let compose = include_str!("../../../assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl");
        let ao = include_str!("../../../assets/shaders/wgpu/hybrid/ambient_occlusion.comp.wgsl");
        let ssr = include_str!("../../../assets/shaders/wgpu/hybrid/ssr.comp.wgsl");
        let rt = include_str!("../../../assets/shaders/wgpu/experimental/hybrid_effects/rt_ao.comp.wgsl");
        let gi = include_str!("../../../assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl");
        assert!(
            ao.contains("ao") && ao.contains("textureStore"),
            "AO grayscale smoke path missing"
        );
        assert!(
            ssr.contains("confidence") && ssr.contains("textureStore"),
            "SSR confidence/color smoke path missing"
        );
        assert!(
            rt.contains("rt_debug_view") || compose.contains("rt_reflection"),
            "RT reflection debug smoke path missing"
        );
        assert!(
            gi.contains("resolved") || gi.contains("out_gi"),
            "resolved GI smoke path missing"
        );
        assert!(
            compose.contains("final") || compose.contains("composite"),
            "final composite smoke path missing"
        );
    }

    const STANDALONE_HYBRID_WGSL: &[(&str, &str)] = &[
        (
            "ambient_occlusion.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/ambient_occlusion.comp.wgsl"),
        ),
        (
            "ssr.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/ssr.comp.wgsl"),
        ),
        (
            "rt_ao.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/experimental/hybrid_effects/rt_ao.comp.wgsl"),
        ),
        (
            "rt_gi.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/experimental/hybrid_effects/rt_gi.comp.wgsl"),
        ),
        (
            "gi_resolve.comp.wgsl",
            include_str!("../../../assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl"),
        ),
        (
            "hybrid_compose.comp.wgsl",
            concat!(
                include_str!("../../../assets/shaders/wgpu/hybrid/pbr_lighting.wgsl"),
                "\n",
                include_str!("../../../assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl")
            ),
        ),
    ];

    const HYBRID_BINDING_LAYOUTS: &[(&str, &str, &[(u32, u32)])] = &[
        (
            "hybrid_compose",
            STANDALONE_HYBRID_WGSL[5].1,
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
                (0, 8),
                (0, 9),
                (0, 10),
                (0, 11),
                (0, 12),
                (0, 13),
                (0, 14),
            ],
        ),
        (
            "ambient_occlusion",
            STANDALONE_HYBRID_WGSL[0].1,
            &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5)],
        ),
        (
            "ssr",
            STANDALONE_HYBRID_WGSL[1].1,
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
            ],
        ),
        (
            "rt_ao",
            STANDALONE_HYBRID_WGSL[2].1,
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
                (0, 8),
                (0, 9),
                (0, 10),
                (0, 11),
                (0, 12),
                (0, 13),
                (0, 14),
            ],
        ),
        (
            "rt_gi",
            STANDALONE_HYBRID_WGSL[3].1,
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
                (0, 8),
                (0, 9),
                (0, 10),
                (0, 11),
                (0, 12),
                (0, 13),
            ],
        ),
        (
            "gi_resolve",
            STANDALONE_HYBRID_WGSL[4].1,
            &[
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (0, 5),
                (0, 6),
                (0, 7),
                (0, 8),
                (0, 9),
                (0, 10),
                (0, 11),
                (0, 12),
                (0, 13),
                (0, 14),
                (0, 15),
                (0, 16),
                (0, 17),
                (0, 18),
            ],
        ),
    ];

    fn wgsl_bindings(source: &str) -> Vec<(u32, u32)> {
        source
            .lines()
            .filter_map(|line| {
                let g = line
                    .split("@group(")
                    .nth(1)?
                    .split(')')
                    .next()?
                    .parse()
                    .ok()?;
                let b = line
                    .split("@binding(")
                    .nth(1)?
                    .split(')')
                    .next()?
                    .parse()
                    .ok()?;
                Some((g, b))
            })
            .collect()
    }

    fn wgsl_struct_fields<'a>(source: &'a str, struct_name: &str) -> Vec<&'a str> {
        let struct_start = source
            .find(&format!("struct {struct_name} {{"))
            .unwrap_or_else(|| panic!("{struct_name} struct not found"));
        let body_start = source[struct_start..]
            .find('{')
            .map(|offset| struct_start + offset + 1)
            .unwrap();
        let body_end = source[body_start..]
            .find("};")
            .map(|offset| body_start + offset)
            .unwrap();

        source[body_start..body_end]
            .lines()
            .flat_map(|line| {
                line.split_once("//")
                    .map_or(line, |(code, _)| code)
                    .split(',')
            })
            .filter_map(|field| field.split_once(':').map(|(name, _)| name.trim()))
            .filter(|name| !name.is_empty())
            .collect()
    }
}
