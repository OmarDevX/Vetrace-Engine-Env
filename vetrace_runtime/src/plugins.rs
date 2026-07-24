use vetrace_core::{AppBuilder, Plugin};
use vetrace_project::RenderingBackend;
use vetrace_render::RenderPlugin;

use crate::{RuntimeConfig, RuntimeError, RuntimeResult, VetraceProject};

pub(crate) fn install_standard_plugins(
    mut builder: AppBuilder,
    project: &VetraceProject,
    config: &RuntimeConfig,
    extra_plugins: Vec<Box<dyn Plugin>>,
) -> RuntimeResult<AppBuilder> {
    let features = &project.manifest().features;

    if features.rendering {
        builder = builder.add_plugin(select_render_plugin(project, config)?);
        #[cfg(feature = "render_2d")]
        {
            builder = builder.add_plugin(vetrace_render::Render2dPlugin::new());
        }
    }
    if features.ui {
        builder = builder.add_plugin(vetrace_ui::UiPlugin::new());
    }
    if features.networking {
        builder = builder.add_plugin(vetrace_net::NetPlugin::new());
    }
    if features.physics {
        builder = builder.add_plugin(vetrace_physics::RapierPhysicsPlugin::new());
        #[cfg(feature = "physics_2d")]
        {
            builder = builder.add_plugin(vetrace_physics::Physics2dPlugin::new());
        }
    }
    if features.animation {
        builder = builder.add_plugin(vetrace_animation::AnimationPlugin::new());
    }
    if features.audio {
        builder = builder.add_plugin(vetrace_audio::AudioPlugin::new());
    }
    if features.scripting {
        let plugin = if config.run_project_scripts {
            vetrace_scripting_lua::LuaScriptingPlugin::new()
        } else {
            vetrace_scripting_lua::LuaScriptingPlugin::authoring_only()
        };
        builder = builder.add_plugin(plugin);
    }

    for plugin in extra_plugins {
        builder = builder.add_boxed_plugin(plugin);
    }
    Ok(builder)
}

fn select_render_plugin(project: &VetraceProject, config: &RuntimeConfig) -> RuntimeResult<RenderPlugin> {
    if config.mode.is_headless() {
        return Ok(RenderPlugin::headless());
    }

    match project.manifest().rendering.backend {
        RenderingBackend::Auto => {
            #[cfg(any(feature = "window", feature = "software_window"))]
            {
                Ok(RenderPlugin::new())
            }
            #[cfg(not(any(feature = "window", feature = "software_window")))]
            {
                Err(RuntimeError::FeatureUnavailable {
                    feature: "a window renderer",
                    required_cargo_feature: "window or software_window",
                })
            }
        }
        RenderingBackend::Wgpu => {
            #[cfg(feature = "window")]
            {
                Ok(RenderPlugin::wgpu_window_from_settings())
            }
            #[cfg(not(feature = "window"))]
            {
                Err(RuntimeError::FeatureUnavailable {
                    feature: "the WGPU window renderer",
                    required_cargo_feature: "window",
                })
            }
        }
        RenderingBackend::SoftwareSdl => {
            #[cfg(feature = "software_window")]
            {
                Ok(RenderPlugin::sdl_window_from_settings())
            }
            #[cfg(not(feature = "software_window"))]
            {
                Err(RuntimeError::FeatureUnavailable {
                    feature: "the SDL software window renderer",
                    required_cargo_feature: "software_window",
                })
            }
        }
    }
}
