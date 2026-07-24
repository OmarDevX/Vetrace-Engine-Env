use vetrace_core::Engine;
use vetrace_project::VetraceProject;
use vetrace_scene::{load_scene_file, SceneTextureLoadReport};

use crate::{
    scripting::{bind_scene_scripts, reset_scene_script_state},
    ActiveRuntimeScene, RuntimeConfig, RuntimeError, RuntimeResult,
};

pub(crate) fn load_main_scene(
    engine: &mut Engine,
    project: &VetraceProject,
    config: &RuntimeConfig,
) -> RuntimeResult<ActiveRuntimeScene> {
    let authored_path = project.manifest().runtime.main_scene.clone();
    let scene_path = project.paths().resolve_existing(&authored_path)?;
    let document = load_scene_file(&scene_path).map_err(|error| RuntimeError::SceneLoad {
        path: scene_path.clone(),
        message: error.to_string(),
    })?;

    let (instance, textures) = if config.load_scene_assets && project.manifest().features.rendering {
        document
            .instantiate_with_assets(engine, &scene_path)
            .map_err(|error| RuntimeError::SceneLoad {
                path: scene_path.clone(),
                message: error.to_string(),
            })?
    } else {
        let instance = document.instantiate(engine).map_err(|error| RuntimeError::SceneLoad {
            path: scene_path.clone(),
            message: error.to_string(),
        })?;
        (instance, SceneTextureLoadReport::default())
    };

    let entities = instance.actors.iter().map(|actor| actor.entity()).collect::<Vec<_>>();
    if config.run_project_scripts {
        if let Err(error) = bind_scene_scripts(engine, project, &entities) {
            reset_scene_script_state(engine, &entities);
            instance.clone().unload(engine);
            return Err(error);
        }
    }

    Ok(ActiveRuntimeScene {
        path: authored_path,
        document,
        instance,
        textures,
    })
}

pub(crate) fn unload_active_scene(engine: &mut Engine) -> RuntimeResult<ActiveRuntimeScene> {
    let active = engine
        .remove_resource::<ActiveRuntimeScene>()
        .ok_or(RuntimeError::SceneNotLoaded)?;
    let entities = active.instance.actors.iter().map(|actor| actor.entity()).collect::<Vec<_>>();
    reset_scene_script_state(engine, &entities);
    active.instance.clone().unload(engine);
    Ok(active)
}
