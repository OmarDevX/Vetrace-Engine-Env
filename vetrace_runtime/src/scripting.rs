use std::collections::HashMap;

use vetrace_core::{Engine, Entity};
use vetrace_project::{ProjectPath, VetraceProject};
use vetrace_scripting_lua::{
    ScriptComponent, attach_autoload_script, attach_loaded_script, load_script_from_file_as,
    shutdown_entity_scripts,
};

use crate::{RuntimeAutoloads, RuntimeError, RuntimeResult};

pub(crate) fn load_autoload_scripts(engine: &mut Engine, project: &VetraceProject) -> RuntimeResult<()> {
    if !project.manifest().features.scripting {
        engine.insert_resource(RuntimeAutoloads::default());
        return Ok(());
    }

    let mut loaded = Vec::new();
    for script in &project.manifest().runtime.autoload_scripts {
        let path = project.paths().resolve_existing(script)?;
        let key = script.as_str().to_owned();
        load_script_from_file_as(engine, &path, key.clone()).map_err(|error| RuntimeError::ScriptLoad {
            script: script.clone(),
            message: error.to_string(),
        })?;
        attach_autoload_script(engine, key);
        loaded.push(script.clone());
    }
    engine.insert_resource(RuntimeAutoloads { scripts: loaded });
    Ok(())
}

pub(crate) fn bind_scene_scripts(
    engine: &mut Engine,
    project: &VetraceProject,
    entities: &[Entity],
) -> RuntimeResult<()> {
    if !project.manifest().features.scripting { return Ok(()); }

    let authored = entities
        .iter()
        .copied()
        .filter_map(|entity| {
            engine
                .raw_world()
                .get::<ScriptComponent>(entity)
                .cloned()
                .map(|component| (entity, component))
        })
        .collect::<Vec<_>>();
    let mut loaded_paths: HashMap<ProjectPath, String> = HashMap::new();

    for (entity, component) in authored {
        let reference = ProjectPath::new(&component.script).map_err(|error| RuntimeError::ScriptBinding {
            entity: entity.0,
            script: component.script.clone(),
            message: error.to_string(),
        })?;
        if reference.extension() != Some("lua") || !reference.starts_with("assets/scripts") {
            return Err(RuntimeError::ScriptBinding {
                entity: entity.0,
                script: component.script,
                message: "scene scripts must be .lua files under assets/scripts/".to_owned(),
            });
        }

        let key = if let Some(key) = loaded_paths.get(&reference) {
            key.clone()
        } else {
            let path = project.paths().resolve_existing(&reference)?;
            let key = reference.as_str().to_owned();
            load_script_from_file_as(engine, &path, key.clone()).map_err(|error| RuntimeError::ScriptLoad {
                script: reference.clone(),
                message: error.to_string(),
            })?;
            loaded_paths.insert(reference, key.clone());
            key
        };
        attach_loaded_script(engine, entity, key);
    }
    Ok(())
}

pub(crate) fn reset_scene_script_state(engine: &mut Engine, entities: &[Entity]) {
    shutdown_entity_scripts(engine, entities);
}
