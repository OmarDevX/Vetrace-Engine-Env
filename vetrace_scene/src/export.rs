use std::collections::{BTreeMap, HashSet};

use uuid::Uuid;
use vetrace_core::{ActorId, Engine, Entity, Metadata, Parent};

use crate::SceneNode;

pub(crate) fn export_roots_from_engine(engine: &Engine) -> Vec<SceneNode> {
    let mut child_map: BTreeMap<Entity, Vec<Entity>> = BTreeMap::new();
    let mut child_set = HashSet::new();
    for entity in engine.raw_world().entities() {
        if is_helper(engine, entity) { continue; }
        if let Some(parent) = engine.raw_world().get::<Parent>(entity) {
            if engine.raw_world().is_alive(parent.0) && !is_helper(engine, parent.0) {
                child_map.entry(parent.0).or_default().push(entity);
                child_set.insert(entity);
            }
        }
    }

    let mut roots = Vec::new();
    for entity in engine.raw_world().entities() {
        if child_set.contains(&entity) || is_helper(engine, entity) { continue; }
        if let Some(node) = export_node_recursive(engine, entity, &child_map) {
            roots.push(node);
        }
    }
    roots.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    roots
}

fn export_node_recursive(engine: &Engine, entity: Entity, child_map: &BTreeMap<Entity, Vec<Entity>>) -> Option<SceneNode> {
    let mut node = SceneNode::from_entity(engine, entity)?;
    if let Some(children) = child_map.get(&entity) {
        for child in children {
            if let Some(child_node) = export_node_recursive(engine, *child, child_map) {
                node.children.push(child_node);
            }
        }
        node.children.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    }
    Some(node)
}

pub(crate) fn has_exportable_children(engine: &Engine, entity: Entity) -> bool {
    engine.raw_world().query::<Parent>()
        .into_iter()
        .any(|(child, parent)| parent.0 == entity && !is_helper(engine, child))
}

pub(crate) fn export_tags(engine: &Engine, entity: Entity) -> Vec<String> {
    engine.raw_world().get::<Metadata>(entity)
        .map(|metadata| metadata.tags.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|tag| tag != vetrace_primitives::tags::MAP_BUILDER_HELPER)
        .collect()
}

pub(crate) fn is_helper(engine: &Engine, entity: Entity) -> bool {
    engine.raw_world().get::<Metadata>(entity)
        .map(|metadata| metadata.tags.iter().any(|tag| tag == vetrace_primitives::tags::MAP_BUILDER_HELPER))
        .unwrap_or(false)
}

pub(crate) fn stable_or_random_id(engine: &Engine, entity: Entity) -> String {
    if let Some(id) = engine.raw_world().get::<ActorId>(entity) { return id.to_string(); }
    if let Some(metadata) = engine.raw_world().get::<Metadata>(entity) {
        if let Some(id) = metadata.tags.iter().find_map(|tag| tag.strip_prefix("scene_id:")) {
            return id.to_string();
        }
        if let Some(id) = metadata.tags.iter().find_map(|tag| tag.strip_prefix("prefab_id:")) {
            return id.to_string();
        }
    }
    Uuid::new_v4().to_string()
}
