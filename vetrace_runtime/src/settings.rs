use vetrace_core::{Engine, FixedTime};
use vetrace_project::{
    AdapterPreference as ProjectAdapterPreference, AmbientOcclusion, AntiAliasing, GiMode,
    PresentMode, ShadowQuality, VetraceProject,
};
use vetrace_render::{
    AdapterPreference, AmbientOcclusionMode, AntiAliasingMode, GlobalIlluminationMode,
    PostProcessing, PresentModePreference, RenderSettings, ShadowFilterMode,
};
use vetrace_scripting_lua::{LuaInputAction, LuaInputMap, LuaRuntimeConfig};

use crate::{RuntimeConfig, RuntimeDiagnostics, RuntimeInputMap, RuntimeMode};

pub(crate) fn install_project_resources(
    engine: &mut Engine,
    project: &VetraceProject,
    config: &RuntimeConfig,
) {
    let manifest = project.manifest();

    engine.insert_resource(RuntimeInputMap::new(manifest.input.clone()));
    if manifest.features.scripting {
        let actions = manifest
            .input
            .actions
            .iter()
            .map(|(name, action)| {
                (
                    name.clone(),
                    LuaInputAction {
                        keys: action.keys.clone(),
                        mouse_buttons: action.mouse_buttons.clone(),
                    },
                )
            })
            .collect();
        engine.insert_resource(LuaInputMap::new(actions));
        engine.insert_resource(LuaRuntimeConfig {
            fail_fast: manifest.scripting.fail_fast,
            max_errors_per_frame: manifest.scripting.max_errors_per_frame,
        });
    }
    let mut fixed_time = FixedTime::new(manifest.physics.fixed_timestep);
    fixed_time.max_steps_per_frame = manifest.physics.max_substeps as usize;
    engine.insert_resource(fixed_time);

    apply_project_render_settings(engine, project, config.mode);

    let mut diagnostics = RuntimeDiagnostics::default();
    if manifest.features.scripting
        && manifest
            .input
            .actions
            .values()
            .any(|action| !action.gamepad_buttons.is_empty() || !action.axes.is_empty())
    {
        diagnostics.push_warning(
            "Lua input actions currently map keyboard and mouse bindings; gamepad buttons and axes are preserved for a later input backend bridge",
        );
    }
    if !manifest.application.resizable {
        diagnostics.push_warning(
            "application.resizable is preserved but the current window backends do not yet enforce it",
        );
    }
    if manifest.application.fullscreen {
        diagnostics.push_warning(
            "application.fullscreen is preserved but the current window backends do not yet enforce it",
        );
    }
    if manifest.features.rendering {
        if manifest.rendering.hdr {
            diagnostics.push_warning("rendering.hdr is not yet mapped to the active renderer");
        }
        if manifest.rendering.msaa_samples > 1 {
            diagnostics.push_warning(format!(
                "rendering.msaa_samples={} is not supported by the current renderer; use rendering.anti_aliasing=\"fxaa\" instead",
                manifest.rendering.msaa_samples
            ));
        }
        if (manifest.rendering.render_scale - 1.0).abs() > f32::EPSILON {
            diagnostics.push_warning(
                "rendering.render_scale is not yet mapped to the active renderer",
            );
        }
        match manifest.rendering.gi_mode {
            GiMode::None => {}
            GiMode::Baked => diagnostics.push_warning(
                "rendering.gi_mode=\"baked\" uses authored lightmaps/probes when present and the renderer ambient fallback otherwise",
            ),
            GiMode::Ddgi => diagnostics.push_warning(
                "rendering.gi_mode=\"ddgi\" is routed to the active renderer; this lightweight player currently uses its ambient fallback until a DDGI probe-volume backend is installed",
            ),
        }
    }
    if manifest.features.audio && !cfg!(feature = "audio_backend") {
        diagnostics.push_warning(
            "audio is enabled by the project, but this runtime build has no audio_backend feature",
        );
    }
    if config.mode == RuntimeMode::EditorPreview && !manifest.features.rendering {
        diagnostics.push_warning("editor preview is running without the project rendering feature");
    }
    engine.insert_resource(diagnostics);
}

/// Applies project rendering quality to the live renderer resources.
///
/// Editor preview keeps its own window title, size, and cursor policy after the
/// window exists, while quality controls such as shadows, AO, and GI update
/// immediately. Standalone runtimes receive the complete application policy at
/// startup.
pub(crate) fn apply_project_render_settings(
    engine: &mut Engine,
    project: &VetraceProject,
    mode: RuntimeMode,
) {
    let manifest = project.manifest();
    if !manifest.features.rendering {
        return;
    }

    let existing = engine.get_resource::<RenderSettings>().cloned();
    let preserve_editor_window = existing.is_some()
        && matches!(mode, RuntimeMode::EditorPreview | RuntimeMode::Test);
    let mut settings = existing.unwrap_or_default();
    let rendering = &manifest.rendering;

    if !preserve_editor_window {
        settings.title = manifest.application.title.clone();
        settings.width = manifest.application.width;
        settings.height = manifest.application.height;
    }

    settings.present_mode = match rendering.present_mode {
        PresentMode::Auto => {
            if rendering.vsync {
                PresentModePreference::Vsync
            } else {
                PresentModePreference::LowLatency
            }
        }
        PresentMode::Vsync => PresentModePreference::Vsync,
        PresentMode::LowLatency => PresentModePreference::LowLatency,
        PresentMode::Immediate => PresentModePreference::Immediate,
        PresentMode::Mailbox => PresentModePreference::Mailbox,
        PresentMode::Fifo => PresentModePreference::Fifo,
    };
    settings.adapter_preference = match rendering.adapter_preference {
        ProjectAdapterPreference::LowPower => AdapterPreference::LowPower,
        ProjectAdapterPreference::HighPerformance => AdapterPreference::HighPerformance,
    };
    settings.anti_aliasing_mode = match rendering.anti_aliasing {
        AntiAliasing::Off => AntiAliasingMode::Off,
        AntiAliasing::Fxaa => AntiAliasingMode::Fxaa,
    };
    settings.ambient_occlusion_mode = match rendering.ambient_occlusion {
        AmbientOcclusion::Off => AmbientOcclusionMode::Off,
        AmbientOcclusion::Ssao => AmbientOcclusionMode::Ssao,
    };
    settings.ssao_radius_pixels = rendering.ssao_radius_pixels;
    settings.ssao_intensity = rendering.ssao_intensity;
    settings.ssao_sample_count = rendering.ssao_sample_count;
    settings.shadow_max_distance = rendering.shadow_max_distance;
    settings.shadow_soft_radius = rendering.shadow_soft_radius;
    settings.shadow_bias = rendering.shadow_bias;
    settings.shadow_slope_bias = rendering.shadow_slope_bias;
    settings.shadow_normal_bias = rendering.shadow_normal_bias;
    settings.shadow_cache_geometry = rendering.shadow_cache_geometry;

    let editor_cursor = matches!(mode, RuntimeMode::EditorPreview | RuntimeMode::Test);
    if editor_cursor {
        settings.cursor_grab = false;
        settings.cursor_visible = true;
    } else {
        settings.cursor_grab = manifest.application.cursor_grab;
        settings.cursor_visible = manifest.application.cursor_visible;
    }
    apply_shadow_quality(&mut settings, rendering.shadow_quality);
    engine.insert_resource(settings);

    let mut post_processing = engine
        .get_resource::<PostProcessing>()
        .cloned()
        .unwrap_or_default();
    post_processing.gi_mode = map_gi_mode(rendering.gi_mode);
    engine.insert_resource(post_processing);
}

pub(crate) fn apply_post_plugin_settings(engine: &mut Engine, project: &VetraceProject) {
    if project.manifest().features.physics {
        if let Some(state) = engine.get_resource_mut::<vetrace_physics::PhysicsState>() {
            let gravity = project.manifest().physics.gravity;
            state.gravity.x = gravity[0];
            state.gravity.y = gravity[1];
            state.gravity.z = gravity[2];
            state.integration_parameters.dt = project.manifest().physics.fixed_timestep;
        }
        #[cfg(feature = "physics_2d")]
        if let Some(state) = engine.get_resource_mut::<vetrace_physics::Physics2dState>() {
            let gravity = project.manifest().physics.gravity;
            state.gravity.x = gravity[0];
            state.gravity.y = gravity[1];
            state.max_substeps = project.manifest().physics.max_substeps.max(1) as usize;
        }
    }
}

fn map_gi_mode(mode: GiMode) -> GlobalIlluminationMode {
    match mode {
        GiMode::None => GlobalIlluminationMode::Off,
        GiMode::Baked => GlobalIlluminationMode::Ambient,
        GiMode::Ddgi => GlobalIlluminationMode::Ddgi,
    }
}

fn apply_shadow_quality(settings: &mut RenderSettings, quality: ShadowQuality) {
    match quality {
        ShadowQuality::Off => {
            settings.shadow_max_vertices = 0;
            settings.shadow_filter_mode = ShadowFilterMode::Hard;
        }
        ShadowQuality::Low => {
            settings.shadow_max_vertices = 60_000;
            settings.shadow_map_size = 1024;
            settings.shadow_cascade_count = 1;
            settings.shadow_pcf_quality = 1;
            settings.shadow_filter_mode = ShadowFilterMode::Pcf;
        }
        ShadowQuality::Medium => {
            settings.shadow_max_vertices = 120_000;
            settings.shadow_map_size = 2048;
            settings.shadow_cascade_count = 2;
            settings.shadow_pcf_quality = 2;
            settings.shadow_filter_mode = ShadowFilterMode::Pcss;
        }
        ShadowQuality::High => {
            settings.shadow_max_vertices = 250_000;
            settings.shadow_map_size = 4096;
            settings.shadow_cascade_count = 3;
            settings.shadow_pcf_quality = 3;
            settings.shadow_filter_mode = ShadowFilterMode::Pcss;
        }
        ShadowQuality::Ultra => {
            settings.shadow_max_vertices = 500_000;
            settings.shadow_map_size = 4096;
            settings.shadow_cascade_count = 4;
            settings.shadow_pcf_quality = 3;
            settings.shadow_filter_mode = ShadowFilterMode::EvsmBlurred;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_quality_can_be_reenabled_after_off() {
        let mut settings = RenderSettings::default();
        apply_shadow_quality(&mut settings, ShadowQuality::Off);
        assert_eq!(settings.shadow_max_vertices, 0);
        assert_eq!(settings.shadow_filter_mode, ShadowFilterMode::Hard);

        apply_shadow_quality(&mut settings, ShadowQuality::Ultra);
        assert_eq!(settings.shadow_max_vertices, 500_000);
        assert_eq!(settings.shadow_map_size, 4096);
        assert_eq!(settings.shadow_cascade_count, 4);
        assert_eq!(settings.shadow_filter_mode, ShadowFilterMode::EvsmBlurred);
    }

    #[test]
    fn project_gi_modes_map_to_renderer_modes() {
        assert!(matches!(map_gi_mode(GiMode::None), GlobalIlluminationMode::Off));
        assert!(matches!(
            map_gi_mode(GiMode::Baked),
            GlobalIlluminationMode::Ambient
        ));
        assert!(matches!(
            map_gi_mode(GiMode::Ddgi),
            GlobalIlluminationMode::Ddgi
        ));
    }
}
