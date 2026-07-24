use super::*;

// Split-out implementation details for `wgpu_backend.rs`.

use crate::components::{CustomShaderMaterial, Outline};
use crate::resources::RenderSettings;

const MAX_SHADER_PARAMS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CustomShaderUniform {
    /// Four vec4 lanes avoid WGSL uniform-array stride surprises.
    pub params: [[f32; 4]; 4],
    pub color_a: [f32; 4],
    pub color_b: [f32; 4],
    pub time_health: [f32; 4],
    /// xyz = direction the directional light travels through the scene,
    /// w = intensity.  Appended after the original fields so older custom
    /// WGSL shaders that only read the prefix keep working.
    pub light_direction_intensity: [f32; 4],
    /// rgb = main directional-light color, a = ambient floor.
    pub light_color_ambient: [f32; 4],
    /// x = roughness factor, y = metallic factor, z = alpha, w = flags.
    pub pbr_params: [f32; 4],
    /// x = normal map scale, y = occlusion strength, z = alpha cutoff, w = alpha mode (0 opaque, 1 mask, 2 blend).
    pub pbr_extra: [f32; 4],
    /// x = directional count, y = point count, z = spot count, w = ambient floor.
    pub light_counts: [f32; 4],
    /// xyz = world-space direction the light travels, w = intensity.
    pub directional_lights: [[f32; 4]; 4],
    /// rgb = color, w unused.
    pub directional_colors: [[f32; 4]; 4],
    /// xyz = position, w = intensity.
    pub point_lights: [[f32; 4]; 8],
    /// rgb = color, w = range; range <= 0 means unlimited.
    pub point_colors_ranges: [[f32; 4]; 8],
    /// xyz = position, w = intensity.
    pub spot_lights: [[f32; 4]; 4],
    /// xyz = world-space emission direction, w = range; range <= 0 means unlimited.
    pub spot_dirs_ranges: [[f32; 4]; 4],
    /// rgb = color, w = cos(inner cone angle).
    pub spot_colors_inner: [[f32; 4]; 4],
    /// x = cos(outer cone angle), yzw unused.
    pub spot_params: [[f32; 4]; 4],
    /// Legacy/first cascade directional shadow light view-projection matrix.
    /// Kept for older internal shaders; default material uses the cascade array.
    pub shadow_view_proj: [[f32; 4]; 4],
    /// x = enabled, y = map size, z = constant bias, w = soft PCF radius in texels.
    pub shadow_params: [f32; 4],
    /// Cascaded directional shadow matrices. Unused lanes are identity.
    pub shadow_cascade_view_proj: [[[f32; 4]; 4]; 4],
    /// Per-cascade camera-distance split ends. Unused lanes are large.
    pub shadow_cascade_splits: [f32; 4],
    /// x = cascade count, y = PCF quality, z = ShadowFilterMode (0 hard, 1 PCF, 2 PCSS, 3 EVSM-blurred), w = PCSS light radius.
    pub shadow_extra: [f32; 4],
    /// x = slope-scale bias, y = normal offset bias, z = EVSM blur radius, w = EVSM exponent.
    pub shadow_bias_extra: [f32; 4],
    /// Local-to-world transform. Scene and shadow vertex shaders now transform on the GPU
    /// so meshes can stay in persistent local-space buffers instead of being rebuilt every frame.
    pub model: [[f32; 4]; 4],
    /// Inverse-transpose model matrix for normals/tangents. Stored as mat4 for WGSL uniform alignment.
    pub normal_model: [[f32; 4]; 4],
    /// rgb = fog albedo/tint, a = density in world units.
    /// Appended after model data so existing shader offsets stay stable.
    pub fog_color_density: [f32; 4],
    /// x = enabled, y = anisotropy, z = sky fog distance, w = reserved.
    pub fog_params: [f32; 4],
    /// xy = lightmap UV scale, zw = lightmap UV offset.
    pub baked_lightmap_transform: [f32; 4],
    /// x = lightmap enabled, y = probes enabled, z = reserved, w = lightmap intensity.
    pub baked_gi_params: [f32; 4],
    /// x = baked debug mode, y = baked runtime mode, zw reserved.
    pub baked_gi_extra: [f32; 4],
    /// L2 spherical-harmonic irradiance coefficients for baked probes.
    pub baked_probe_sh0: [f32; 4],
    pub baked_probe_sh1: [f32; 4],
    pub baked_probe_sh2: [f32; 4],
    pub baked_probe_sh3: [f32; 4],
    pub baked_probe_sh4: [f32; 4],
    pub baked_probe_sh5: [f32; 4],
    pub baked_probe_sh6: [f32; 4],
    pub baked_probe_sh7: [f32; 4],
    pub baked_probe_sh8: [f32; 4],
    /// x = exposure multiplier, y = output gamma, z = tone mapper mode, w reserved.
    pub post_process_params: [f32; 4],
    /// Up to four scene reflection-probe indices. `u32::MAX` marks an unused lane.
    pub reflection_probe_indices: [u32; 4],
    /// x = selected probe count, yzw reserved.
    pub reflection_probe_params: [f32; 4],
}

impl CustomShaderUniform {
    pub fn from_material(material: &CustomShaderMaterial, settings: &RenderSettings) -> Self {
        let mut params = [[0.0_f32; 4]; 4];
        for (index, value) in material.params.iter().copied().take(MAX_SHADER_PARAMS).enumerate() {
            params[index / 4][index % 4] = value;
        }
        let health = material.params.get(1).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        Self {
            params,
            color_a: [material.fallback_color_a.x, material.fallback_color_a.y, material.fallback_color_a.z, 1.0],
            color_b: [material.fallback_color_b.x, material.fallback_color_b.y, material.fallback_color_b.z, 1.0],
            time_health: [settings.time_seconds, health, 0.0, 0.0],
            light_direction_intensity: [-0.35, -1.0, -0.25, 1.0],
            light_color_ambient: [1.0, 1.0, 1.0, 0.35],
            pbr_params: [0.5, 0.0, 1.0, 0.0],
            pbr_extra: [1.0, 1.0, 0.5, 0.0],
            light_counts: [1.0, 0.0, 0.0, 0.35],
            directional_lights: [[-0.35, -1.0, -0.25, 1.0], [0.0; 4], [0.0; 4], [0.0; 4]],
            directional_colors: [[1.0, 1.0, 1.0, 0.0], [0.0; 4], [0.0; 4], [0.0; 4]],
            point_lights: [[0.0; 4]; 8],
            point_colors_ranges: [[0.0; 4]; 8],
            spot_lights: [[0.0; 4]; 4],
            spot_dirs_ranges: [[0.0; 4]; 4],
            spot_colors_inner: [[0.0; 4]; 4],
            spot_params: [[0.0; 4]; 4],
            shadow_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            shadow_params: [0.0, 1024.0, 0.0015, 0.0],
            shadow_cascade_view_proj: [glam::Mat4::IDENTITY.to_cols_array_2d(); 4],
            shadow_cascade_splits: [10_000.0; 4],
            shadow_extra: [1.0, 1.0, 0.0, 0.0],
            shadow_bias_extra: [1.0, 0.0, 2.5, 12.0],
            model: Mat4::IDENTITY.to_cols_array_2d(),
            normal_model: Mat4::IDENTITY.to_cols_array_2d(),
            fog_color_density: [0.6, 0.6, 0.6, 0.0],
            fog_params: [0.0, 0.0, 80.0, 0.0],
            baked_lightmap_transform: [1.0, 1.0, 0.0, 0.0],
            baked_gi_params: [0.0, 0.0, 1.0, 1.0],
            baked_gi_extra: [0.0; 4],
            baked_probe_sh0: [0.0; 4],
            baked_probe_sh1: [0.0; 4],
            baked_probe_sh2: [0.0; 4],
            baked_probe_sh3: [0.0; 4],
            baked_probe_sh4: [0.0; 4],
            baked_probe_sh5: [0.0; 4],
            baked_probe_sh6: [0.0; 4],
            baked_probe_sh7: [0.0; 4],
            baked_probe_sh8: [0.0; 4],
            post_process_params: [1.0, 2.2, 1.0, 0.0],
            reflection_probe_indices: [u32::MAX; 4],
            reflection_probe_params: [0.0; 4],
        }
    }

    pub fn set_model(&mut self, model: Mat4) {
        self.model = model.to_cols_array_2d();
        let normal_model = if model.determinant().abs() > 1.0e-8 {
            model.inverse().transpose()
        } else {
            Mat4::IDENTITY
        };
        self.normal_model = normal_model.to_cols_array_2d();
    }

    pub fn set_lighting(&mut self, direction: Vec3, color: Vec3, intensity: f32, ambient: f32) {
        let direction = direction.normalize_or_zero();
        let direction = if direction.length_squared() > 0.0 { direction } else { Vec3::new(-0.35, -1.0, -0.25).normalize() };
        self.light_direction_intensity = [direction.x, direction.y, direction.z, intensity.max(0.0)];
        let color = color.clamp(Vec3::ZERO, Vec3::ONE);
        self.light_color_ambient = [color.x, color.y, color.z, ambient.clamp(0.0, 1.0)];
    }

    pub fn set_pbr(&mut self, roughness: f32, metallic: f32, alpha: f32, flags: f32) {
        self.pbr_params = [roughness.clamp(0.04, 1.0), metallic.clamp(0.0, 1.0), alpha.clamp(0.0, 1.0), flags];
    }

    pub fn set_pbr_extra(&mut self, normal_scale: f32, occlusion_strength: f32, alpha_cutoff: f32, alpha_mode: f32) {
        self.pbr_extra = [
            normal_scale.max(0.0),
            occlusion_strength.clamp(0.0, 1.0),
            alpha_cutoff.clamp(0.0, 1.0),
            alpha_mode.clamp(0.0, 2.0),
        ];
    }


    pub fn set_fog(&mut self, enabled: bool, color: Vec3, density: f32, anisotropy: f32) {
        let color = color.clamp(Vec3::ZERO, Vec3::splat(10.0));
        self.fog_color_density = [color.x, color.y, color.z, density.max(0.0)];
        self.fog_params = [if enabled && density > 0.0 { 1.0 } else { 0.0 }, anisotropy.clamp(-0.95, 0.95), 80.0, 0.0];
    }

    pub fn set_post_process(&mut self, exposure: f32, gamma: f32, tone_mapper: f32) {
        self.post_process_params = [
            exposure.max(0.0001),
            gamma.clamp(1.0, 3.0),
            tone_mapper.clamp(0.0, 3.0),
            0.0,
        ];
    }

    pub fn set_reflection_probes(&mut self, indices: [u32; 4], count: usize) {
        self.reflection_probe_indices = indices;
        self.reflection_probe_params = [count.min(4) as f32, 0.0, 0.0, 0.0];
    }

    pub fn set_baked_lighting(
        &mut self,
        lightmap_transform: Option<glam::Vec4>,
        lightmap_intensity: f32,
        probes: Option<(crate::resources::BakedProbeSample, f32)>,
        debug_mode: crate::resources::BakedLightingDebugMode,
        runtime_mode: crate::resources::BakedLightingRuntimeMode,
        static_lighting_only: bool,
        preserve_local_lights: bool,
    ) {
        self.baked_lightmap_transform = lightmap_transform.unwrap_or(glam::Vec4::new(1.0, 1.0, 0.0, 0.0)).to_array();
        self.baked_gi_params = [
            if lightmap_transform.is_some() { 1.0 } else { 0.0 },
            if probes.is_some() { 1.0 } else { 0.0 },
            1.0,
            lightmap_intensity.max(0.0),
        ];
        let (sample, intensity) = probes.unwrap_or_default();
        let scale = intensity.max(0.0);
        let lane = |value: glam::Vec3| [value.x * scale, value.y * scale, value.z * scale, 0.0];
        self.baked_probe_sh0 = lane(sample.sh_coefficients[0]);
        self.baked_probe_sh1 = lane(sample.sh_coefficients[1]);
        self.baked_probe_sh2 = lane(sample.sh_coefficients[2]);
        self.baked_probe_sh3 = lane(sample.sh_coefficients[3]);
        self.baked_probe_sh4 = lane(sample.sh_coefficients[4]);
        self.baked_probe_sh5 = lane(sample.sh_coefficients[5]);
        self.baked_probe_sh6 = lane(sample.sh_coefficients[6]);
        self.baked_probe_sh7 = lane(sample.sh_coefficients[7]);
        self.baked_probe_sh8 = lane(sample.sh_coefficients[8]);
        self.baked_gi_extra = [debug_mode.shader_value(), runtime_mode.shader_value(), 0.0, 0.0];
        if static_lighting_only
            && runtime_mode == crate::resources::BakedLightingRuntimeMode::BakedOnly
            && lightmap_transform.is_some()
        {
            self.light_counts[0] = 0.0;
            self.light_counts[3] = 0.0;
            self.light_direction_intensity[3] = 0.0;
            self.light_color_ambient[3] = 0.0;
            if !preserve_local_lights {
                self.light_counts[1] = 0.0;
                self.light_counts[2] = 0.0;
            }
        }
    }

    pub fn set_shadow(&mut self, view_proj: glam::Mat4, enabled: bool, map_size: f32, bias: f32, soft_radius: f32) {
        self.set_shadow_cascades(
            [view_proj, glam::Mat4::IDENTITY, glam::Mat4::IDENTITY, glam::Mat4::IDENTITY],
            [10_000.0; 4],
            if enabled { 1 } else { 0 },
            enabled,
            map_size,
            bias,
            soft_radius,
            1.0,
            if soft_radius > 0.25 { 1.0 } else { 0.0 },
            0.0,
            0.0,
            0.0,
            2.5,
            12.0,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_shadow_cascades(
        &mut self,
        view_proj: [glam::Mat4; 4],
        splits: [f32; 4],
        cascade_count: usize,
        enabled: bool,
        map_size: f32,
        bias: f32,
        soft_radius: f32,
        pcf_quality: f32,
        filter_mode: f32,
        pcss_light_radius: f32,
        slope_bias: f32,
        normal_bias: f32,
        evsm_blur_radius: f32,
        evsm_exponent: f32,
    ) {
        let enabled_value = enabled && cascade_count > 0;
        self.shadow_view_proj = view_proj[0].to_cols_array_2d();
        self.shadow_params = [
            if enabled_value { 1.0 } else { 0.0 },
            map_size.max(1.0),
            bias.max(0.0),
            if enabled_value { soft_radius.max(0.0) } else { 0.0 },
        ];
        self.shadow_cascade_view_proj = [
            view_proj[0].to_cols_array_2d(),
            view_proj[1].to_cols_array_2d(),
            view_proj[2].to_cols_array_2d(),
            view_proj[3].to_cols_array_2d(),
        ];
        self.shadow_cascade_splits = splits;
        self.shadow_extra = [
            if enabled_value { cascade_count.clamp(1, 4) as f32 } else { 0.0 },
            pcf_quality.max(1.0),
            if enabled_value { filter_mode.clamp(0.0, 3.0) } else { 0.0 },
            pcss_light_radius.max(0.0),
        ];
        self.shadow_bias_extra = [
            slope_bias.max(0.0),
            normal_bias.max(0.0),
            evsm_blur_radius.max(0.0),
            evsm_exponent.clamp(1.0, 30.0),
        ];
    }
}
