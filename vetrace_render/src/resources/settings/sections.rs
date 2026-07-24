use super::*;

/// Non-breaking logical view of window, presentation, and editor-facing renderer settings.
/// `RenderSettings` keeps its flat serialized fields for project compatibility.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderPresentationSettings {
    pub clear_color: [f32; 4],
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub draw_bounds: bool,
    pub draw_names: bool,
    pub cursor_grab: bool,
    pub cursor_visible: bool,
    pub time_seconds: f32,
    pub present_mode: PresentModePreference,
    pub adapter_preference: AdapterPreference,
    pub anti_aliasing_mode: AntiAliasingMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderShadowSettings {
    pub map_size: u32,
    pub max_vertices: u32,
    pub max_distance: f32,
    pub soft_radius: f32,
    pub bias: f32,
    pub slope_bias: f32,
    pub normal_bias: f32,
    pub cascade_count: u32,
    pub filter_mode: ShadowFilterMode,
    pub pcf_quality: u32,
    pub pcss: bool,
    pub pcss_light_radius: f32,
    pub evsm_blur_radius: f32,
    pub evsm_exponent: f32,
    pub cache_geometry: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderAmbientOcclusionSettings {
    pub mode: AmbientOcclusionMode,
    pub radius_pixels: f32,
    pub intensity: f32,
    pub bias: f32,
    pub sample_count: u32,
    pub blur_radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderReflectionSettings {
    pub max_capture_resolution: u32,
    pub capture_faces_per_frame: u32,
    pub prefilter_sample_count: u32,
    pub capture_probe_budget_per_frame: u32,
    pub prefilter_mips_per_frame: u32,
    pub max_resident_runtime_probes: u32,
    pub capture_distance_limit: f32,
    pub probe_grid_cell_size: f32,
}

impl RenderSettings {
    pub fn presentation_settings(&self) -> RenderPresentationSettings {
        RenderPresentationSettings {
            clear_color: self.clear_color,
            width: self.width,
            height: self.height,
            title: self.title.clone(),
            draw_bounds: self.draw_bounds,
            draw_names: self.draw_names,
            cursor_grab: self.cursor_grab,
            cursor_visible: self.cursor_visible,
            time_seconds: self.time_seconds,
            present_mode: self.present_mode,
            adapter_preference: self.adapter_preference,
            anti_aliasing_mode: self.anti_aliasing_mode,
        }
    }

    pub fn apply_presentation_settings(&mut self, settings: RenderPresentationSettings) {
        self.clear_color = settings.clear_color;
        self.width = settings.width;
        self.height = settings.height;
        self.title = settings.title;
        self.draw_bounds = settings.draw_bounds;
        self.draw_names = settings.draw_names;
        self.cursor_grab = settings.cursor_grab;
        self.cursor_visible = settings.cursor_visible;
        self.time_seconds = settings.time_seconds;
        self.present_mode = settings.present_mode;
        self.adapter_preference = settings.adapter_preference;
        self.anti_aliasing_mode = settings.anti_aliasing_mode;
    }

    pub fn shadow_settings(&self) -> RenderShadowSettings {
        RenderShadowSettings {
            map_size: self.shadow_map_size,
            max_vertices: self.shadow_max_vertices,
            max_distance: self.shadow_max_distance,
            soft_radius: self.shadow_soft_radius,
            bias: self.shadow_bias,
            slope_bias: self.shadow_slope_bias,
            normal_bias: self.shadow_normal_bias,
            cascade_count: self.shadow_cascade_count,
            filter_mode: self.shadow_filter_mode,
            pcf_quality: self.shadow_pcf_quality,
            pcss: self.shadow_pcss,
            pcss_light_radius: self.shadow_pcss_light_radius,
            evsm_blur_radius: self.shadow_evsm_blur_radius,
            evsm_exponent: self.shadow_evsm_exponent,
            cache_geometry: self.shadow_cache_geometry,
        }
    }

    pub fn apply_shadow_settings(&mut self, settings: RenderShadowSettings) {
        self.shadow_map_size = settings.map_size;
        self.shadow_max_vertices = settings.max_vertices;
        self.shadow_max_distance = settings.max_distance;
        self.shadow_soft_radius = settings.soft_radius;
        self.shadow_bias = settings.bias;
        self.shadow_slope_bias = settings.slope_bias;
        self.shadow_normal_bias = settings.normal_bias;
        self.shadow_cascade_count = settings.cascade_count;
        self.shadow_filter_mode = settings.filter_mode;
        self.shadow_pcf_quality = settings.pcf_quality;
        self.shadow_pcss = settings.pcss;
        self.shadow_pcss_light_radius = settings.pcss_light_radius;
        self.shadow_evsm_blur_radius = settings.evsm_blur_radius;
        self.shadow_evsm_exponent = settings.evsm_exponent;
        self.shadow_cache_geometry = settings.cache_geometry;
    }

    pub fn ambient_occlusion_settings(&self) -> RenderAmbientOcclusionSettings {
        RenderAmbientOcclusionSettings {
            mode: self.ambient_occlusion_mode,
            radius_pixels: self.ssao_radius_pixels,
            intensity: self.ssao_intensity,
            bias: self.ssao_bias,
            sample_count: self.ssao_sample_count,
            blur_radius: self.ssao_blur_radius,
        }
    }

    pub fn apply_ambient_occlusion_settings(
        &mut self,
        settings: RenderAmbientOcclusionSettings,
    ) {
        self.ambient_occlusion_mode = settings.mode;
        self.ssao_radius_pixels = settings.radius_pixels;
        self.ssao_intensity = settings.intensity;
        self.ssao_bias = settings.bias;
        self.ssao_sample_count = settings.sample_count;
        self.ssao_blur_radius = settings.blur_radius;
    }

    pub fn reflection_settings(&self) -> RenderReflectionSettings {
        RenderReflectionSettings {
            max_capture_resolution: self.reflection_max_capture_resolution,
            capture_faces_per_frame: self.reflection_capture_faces_per_frame,
            prefilter_sample_count: self.reflection_prefilter_sample_count,
            capture_probe_budget_per_frame: self.reflection_capture_probe_budget_per_frame,
            prefilter_mips_per_frame: self.reflection_prefilter_mips_per_frame,
            max_resident_runtime_probes: self.reflection_max_resident_runtime_probes,
            capture_distance_limit: self.reflection_capture_distance_limit,
            probe_grid_cell_size: self.reflection_probe_grid_cell_size,
        }
    }

    pub fn apply_reflection_settings(&mut self, settings: RenderReflectionSettings) {
        self.reflection_max_capture_resolution = settings.max_capture_resolution;
        self.reflection_capture_faces_per_frame = settings.capture_faces_per_frame;
        self.reflection_prefilter_sample_count = settings.prefilter_sample_count;
        self.reflection_capture_probe_budget_per_frame =
            settings.capture_probe_budget_per_frame;
        self.reflection_prefilter_mips_per_frame = settings.prefilter_mips_per_frame;
        self.reflection_max_resident_runtime_probes = settings.max_resident_runtime_probes;
        self.reflection_capture_distance_limit = settings.capture_distance_limit;
        self.reflection_probe_grid_cell_size = settings.probe_grid_cell_size;
    }
}
