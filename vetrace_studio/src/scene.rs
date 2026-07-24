use std::collections::{BTreeMap, BTreeSet, HashMap};

use vetrace_core::{Actor, ActorId, ComponentSchema, DynamicValue, Engine, Entity, FieldPath, Name, Parent};
use vetrace_editor::{EditorOnly, EditorState};
use vetrace_project::{ProjectPath, VetraceProject};
use vetrace_runtime::ActiveRuntimeScene;
use vetrace_scene::{
    load_scene_file, save_scene_file, SceneDocument, SceneInstance,
};

use crate::protocol::{
    EntityRow, ReflectedComponentSnapshot, ReflectedFieldSnapshot, StudioSnapshot,
};


#[derive(Clone, Debug)]
pub struct AuthoredSceneSnapshot {
    pub document: SceneDocument,
    pub selected_actor_id: Option<ActorId>,
    fingerprint: Vec<u8>,
}

impl AuthoredSceneSnapshot {
    pub fn fingerprint(&self) -> &[u8] { &self.fingerprint }
}

impl PartialEq for AuthoredSceneSnapshot {
    fn eq(&self, other: &Self) -> bool { self.fingerprint == other.fingerprint }
}

pub fn capture_authored_scene(engine: &mut Engine) -> Result<AuthoredSceneSnapshot, String> {
    let scene_name = engine
        .get_resource::<ActiveRuntimeScene>()
        .map(|scene| scene.document.name.clone())
        .unwrap_or_else(|| "Main Scene".to_string());
    let selected_actor_id = engine
        .get_resource::<EditorState>()
        .and_then(EditorState::selected_primary)
        .and_then(|entity| engine.actor(entity))
        .and_then(|actor| actor.id(engine));

    vetrace_editor::prepare_editor_scene_export(engine);
    let document = SceneDocument::from_engine(engine, scene_name);
    vetrace_editor::restore_editor_selection_visuals(engine);
    let fingerprint = serde_json::to_vec(&document)
        .map_err(|error| format!("failed to snapshot authored scene: {error}"))?;
    Ok(AuthoredSceneSnapshot { document, selected_actor_id, fingerprint })
}

pub fn restore_authored_scene(
    engine: &mut Engine,
    project: &VetraceProject,
    snapshot: &AuthoredSceneSnapshot,
) -> Result<(), String> {
    let path = active_scene_project_path(engine, project);
    replace_authored_scene(
        engine,
        project,
        path,
        snapshot.document.clone(),
        snapshot.selected_actor_id,
    )
}

pub fn active_scene_project_path(engine: &Engine, project: &VetraceProject) -> ProjectPath {
    engine
        .get_resource::<ActiveRuntimeScene>()
        .map(|scene| scene.path.clone())
        .unwrap_or_else(|| project.manifest().runtime.main_scene.clone())
}

pub fn open_scene(
    engine: &mut Engine,
    project: &VetraceProject,
    path: ProjectPath,
) -> Result<(), String> {
    validate_scene_project_path(&path)?;
    let absolute = project.paths().resolve_existing(&path).map_err(|error| error.to_string())?;
    let document = load_scene_file(&absolute)
        .map_err(|error| format!("failed to load scene '{}': {error}", absolute.display()))?;
    replace_authored_scene(engine, project, path, document, None)
}

pub fn create_scene(
    engine: &mut Engine,
    project: &VetraceProject,
    path: ProjectPath,
) -> Result<(), String> {
    validate_scene_project_path(&path)?;
    let absolute = project.paths().resolve_for_write(&path).map_err(|error| error.to_string())?;
    if absolute.exists() {
        return Err(format!("scene '{}' already exists", path));
    }
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create scene directory '{}': {error}", parent.display()))?;
    }
    let name = path.file_name()
        .and_then(|name| name.strip_suffix(".vscene"))
        .unwrap_or("New Scene");
    let document = SceneDocument::new(name);
    save_scene_file(&absolute, &document)
        .map_err(|error| format!("failed to create scene '{}': {error}", absolute.display()))?;
    replace_authored_scene(engine, project, path, document, None)
}


pub fn restore_scene_document(
    engine: &mut Engine,
    project: &VetraceProject,
    path: ProjectPath,
    document: SceneDocument,
) -> Result<(), String> {
    validate_scene_project_path(&path)?;
    replace_authored_scene(engine, project, path, document, None)
}

pub fn save_active_scene_as(
    engine: &mut Engine,
    project: &VetraceProject,
    path: ProjectPath,
) -> Result<SceneDocument, String> {
    validate_scene_project_path(&path)?;
    let scene_name = engine
        .get_resource::<ActiveRuntimeScene>()
        .map(|scene| scene.document.name.clone())
        .unwrap_or_else(|| "Scene".to_owned());
    vetrace_editor::prepare_editor_scene_export(engine);
    let document = SceneDocument::from_engine(engine, scene_name);
    let absolute = project.paths().resolve_for_write(&path).map_err(|error| error.to_string())?;
    if let Some(parent) = absolute.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create scene directory '{}': {error}", parent.display()))?;
    }
    let result = save_scene_file(&absolute, &document)
        .map_err(|error| format!("failed to save scene '{}': {error}", absolute.display()));
    vetrace_editor::restore_editor_selection_visuals(engine);
    result?;
    let instance = capture_authored_instance(engine);
    if let Some(active) = engine.get_resource_mut::<ActiveRuntimeScene>() {
        active.path = path;
        active.document = document.clone();
        active.instance = instance;
    }
    Ok(document)
}

fn replace_authored_scene(
    engine: &mut Engine,
    project: &VetraceProject,
    path: ProjectPath,
    document: SceneDocument,
    selected_actor_id: Option<ActorId>,
) -> Result<(), String> {
    vetrace_editor::select_editor_entity(engine, None);
    engine.remove_resource::<ActiveRuntimeScene>();
    clear_authored_entities(engine);
    let absolute = project.paths().resolve(&path);
    let (instance, textures) = document
        .instantiate_with_assets(engine, &absolute)
        .map_err(|error| format!("failed to instantiate scene '{}': {error}", path))?;
    let selected = selected_actor_id
        .and_then(|id| instance.actor_ids.get(&id).copied())
        .map(Actor::entity);
    engine.insert_resource(ActiveRuntimeScene {
        path,
        document,
        instance,
        textures,
    });
    vetrace_editor::select_editor_entity(engine, selected);
    Ok(())
}

fn validate_scene_project_path(path: &ProjectPath) -> Result<(), String> {
    if !path.starts_with("assets/scenes") {
        return Err("scene files must be stored under assets/scenes/".to_owned());
    }
    if path.extension() != Some("vscene") {
        return Err("scene files must use the .vscene extension".to_owned());
    }
    Ok(())
}


pub fn save_active_scene(
    engine: &mut Engine,
    project: &VetraceProject,
) -> Result<SceneDocument, String> {
    let scene_name = engine
        .get_resource::<ActiveRuntimeScene>()
        .map(|scene| scene.document.name.clone())
        .unwrap_or_else(|| "Main Scene".to_string());
    vetrace_editor::prepare_editor_scene_export(engine);
    let document = SceneDocument::from_engine(engine, scene_name);
    let authored_path = active_scene_project_path(engine, project);
    let path = project.paths().resolve_for_write(&authored_path).map_err(|error| error.to_string())?;
    let result = save_scene_file(&path, &document).map_err(|error| {
        format!("failed to save scene `{}`: {error}", path.display())
    });
    vetrace_editor::restore_editor_selection_visuals(engine);
    result?;
    let instance = capture_authored_instance(engine);
    if let Some(active) = engine.get_resource_mut::<ActiveRuntimeScene>() {
        active.document = document.clone();
        active.instance = instance;
    }
    Ok(document)
}

/// Serializes the current editor world to an isolated play-session scene.
/// The authored scene and its dirty state are not modified.
pub fn save_temporary_play_scene(
    engine: &mut Engine,
    project: &VetraceProject,
) -> Result<(ProjectPath, std::path::PathBuf), String> {
    let snapshot = capture_authored_scene(engine)?;
    let project_path = ProjectPath::new(format!(
        "assets/.vetrace/play/session-{}.vscene",
        std::process::id()
    ))
    .map_err(|error| error.to_string())?;
    let path = project
        .paths()
        .resolve_for_write(&project_path)
        .map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create play-session directory: {error}"))?;
    }
    save_scene_file(&path, &snapshot.document)
        .map_err(|error| format!("failed to write temporary play scene: {error}"))?;
    Ok((project_path, path))
}

pub fn reload_active_scene(engine: &mut Engine, project: &VetraceProject) -> Result<(), String> {
    let authored_path = active_scene_project_path(engine, project);
    open_scene(engine, project, authored_path)
}


fn capture_authored_instance(engine: &Engine) -> SceneInstance {
    let actors = engine
        .raw_world()
        .entities()
        .filter(|entity| !engine.raw_world().has::<EditorOnly>(*entity))
        .filter_map(|entity| engine.actor(entity))
        .collect::<Vec<_>>();
    let actor_set = actors.iter().map(|actor| actor.entity()).collect::<BTreeSet<_>>();
    let roots = actors
        .iter()
        .copied()
        .filter(|actor| {
            engine
                .raw_world()
                .get::<Parent>(actor.entity())
                .map(|parent| !actor_set.contains(&parent.0))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    let actor_ids = actors
        .iter()
        .filter_map(|actor| actor.id(engine).map(|id| (id, *actor)))
        .collect::<HashMap<ActorId, Actor>>();
    SceneInstance {
        roots,
        actors,
        scene_ids: HashMap::new(),
        actor_ids,
    }
}

fn clear_authored_entities(engine: &mut Engine) {
    let entities = engine
        .raw_world()
        .entities()
        .filter(|entity| !engine.raw_world().has::<EditorOnly>(*entity))
        .collect::<Vec<_>>();
    for entity in entities {
        if let Some(actor) = engine.actor(entity) {
            actor.despawn(engine);
        }
    }
}

pub fn fill_scene_snapshot(engine: &Engine, snapshot: &mut StudioSnapshot) {
    snapshot.selected = engine
        .get_resource::<EditorState>()
        .and_then(EditorState::selected_primary);
    snapshot.entities = scene_rows(engine);
    snapshot.selected_name = snapshot
        .selected
        .and_then(|entity| engine.actor(entity))
        .and_then(|actor| actor.name(engine).map(str::to_owned))
        .unwrap_or_default();
    snapshot.components.clear();
    snapshot.addable_components.clear();

    let Some(entity) = snapshot.selected else { return; };
    let Some(actor) = engine.actor(entity) else { return; };
    let attached = engine.actor_component_schemas(actor);
    let attached_ids = attached
        .iter()
        .map(|schema| schema.stable_id.clone())
        .collect::<BTreeSet<_>>();
    snapshot.components = attached
        .into_iter()
        .filter_map(|schema| component_snapshot(engine, actor, schema))
        .collect();
    snapshot.components.sort_by(|left, right| {
        left.schema
            .category
            .cmp(&right.schema.category)
            .then_with(|| left.schema.display_name.cmp(&right.schema.display_name))
    });
    snapshot.addable_components = engine
        .registered_component_schemas()
        .into_iter()
        .filter(|schema| schema.constructible && !attached_ids.contains(&schema.stable_id))
        .collect();
    snapshot.addable_components.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
}

fn component_snapshot(
    engine: &Engine,
    actor: Actor,
    schema: ComponentSchema,
) -> Option<ReflectedComponentSnapshot> {
    let value = engine
        .registered_component_value(actor, &schema.stable_id)
        .ok()?;
    let fields = if schema.fields.len() == 1 && schema.fields[0].name == "value" {
        vec![ReflectedFieldSnapshot {
            schema: schema.fields[0].clone(),
            path: FieldPath::root(),
            value,
        }]
    } else {
        schema
            .fields
            .iter()
            .filter_map(|field| {
                let path = FieldPath::root().field(field.name.clone());
                let value = value.get(&path).ok()?.clone();
                Some(ReflectedFieldSnapshot {
                    schema: field.clone(),
                    path,
                    value,
                })
            })
            .collect()
    };
    Some(ReflectedComponentSnapshot { schema, fields })
}

fn scene_rows(engine: &Engine) -> Vec<EntityRow> {
    let entities = engine
        .raw_world()
        .entities()
        .filter(|entity| !engine.raw_world().has::<EditorOnly>(*entity))
        .collect::<Vec<_>>();
    let set = entities.iter().copied().collect::<BTreeSet<_>>();
    let mut children: BTreeMap<Entity, Vec<Entity>> = BTreeMap::new();
    let mut roots = Vec::new();
    for entity in entities {
        match engine.raw_world().get::<Parent>(entity).copied() {
            Some(parent) if set.contains(&parent.0) => {
                children.entry(parent.0).or_default().push(entity);
            }
            _ => roots.push(entity),
        }
    }
    let sort_entities = |values: &mut Vec<Entity>| {
        values.sort_by(|left, right| entity_name(engine, *left).cmp(&entity_name(engine, *right)));
    };
    sort_entities(&mut roots);
    for values in children.values_mut() { sort_entities(values); }

    let mut rows = Vec::new();
    let mut visited = BTreeSet::new();
    for root in roots {
        append_row(engine, root, 0, &children, &mut visited, &mut rows);
    }
    // Corrupt hierarchy data should still remain inspectable instead of
    // recursing forever or hiding cyclic/orphaned actors.
    for entity in set {
        if !visited.contains(&entity) {
            append_row(engine, entity, 0, &children, &mut visited, &mut rows);
        }
    }
    rows
}

fn append_row(
    engine: &Engine,
    entity: Entity,
    depth: usize,
    children: &BTreeMap<Entity, Vec<Entity>>,
    visited: &mut BTreeSet<Entity>,
    rows: &mut Vec<EntityRow>,
) {
    if !visited.insert(entity) { return; }
    rows.push(EntityRow { entity, name: entity_name(engine, entity), depth });
    if let Some(entity_children) = children.get(&entity) {
        for child in entity_children {
            append_row(engine, *child, depth + 1, children, visited, rows);
        }
    }
}

fn entity_name(engine: &Engine, entity: Entity) -> String {
    engine
        .raw_world()
        .get::<Name>(entity)
        .map(|name| name.0.clone())
        .unwrap_or_else(|| format!("Entity {}", entity.raw()))
}

pub fn project_settings(project: &VetraceProject) -> Vec<(String, String)> {
    let manifest = project.manifest();
    vec![
        ("Project".into(), manifest.project.name.clone()),
        ("Version".into(), manifest.project.version.clone()),
        ("Engine".into(), manifest.project.engine_version.clone()),
        ("Main scene".into(), manifest.runtime.main_scene.to_string()),
        ("Window".into(), format!("{} × {}", manifest.application.width, manifest.application.height)),
        ("Rendering".into(), manifest.features.rendering.to_string()),
        ("Physics".into(), manifest.features.physics.to_string()),
        ("Audio".into(), manifest.features.audio.to_string()),
        ("Scripting".into(), manifest.features.scripting.to_string()),
        ("Networking".into(), manifest.features.networking.to_string()),
    ]
}
