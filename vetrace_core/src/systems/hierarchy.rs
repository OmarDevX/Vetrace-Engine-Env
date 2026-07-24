use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;


use crate::app::Plugin;
use crate::components::builtins::{GlobalTransform, Parent, Transform, TransformDirty};
use crate::ecs::Entity;
use crate::engine::Engine;
use crate::Stage;

fn combine(parent: &GlobalTransform, local: &Transform) -> GlobalTransform {
    GlobalTransform {
        translation: parent.translation + parent.rotation * (local.translation * parent.scale),
        rotation: parent.rotation * local.rotation,
        scale: parent.scale * local.scale,
    }
}

fn compute_entity(
    entity: Entity,
    locals: &HashMap<Entity, Transform>,
    parents: &HashMap<Entity, Entity>,
    cache: &mut HashMap<Entity, GlobalTransform>,
    stack: &mut Vec<Entity>,
) -> GlobalTransform {
    if let Some(global) = cache.get(&entity) {
        return global.clone();
    }

    if stack.contains(&entity) {
        // Cycle guard: keep the entity local rather than recursing forever.
        return locals.get(&entity).map(GlobalTransform::from).unwrap_or_default();
    }

    stack.push(entity);
    let local = locals.get(&entity).cloned().unwrap_or_default();
    let global = if let Some(parent) = parents.get(&entity).copied() {
        let parent_global = compute_entity(parent, locals, parents, cache, stack);
        combine(&parent_global, &local)
    } else {
        GlobalTransform::from(&local)
    };
    stack.pop();
    cache.insert(entity, global.clone());
    global
}

/// Rebuilds `GlobalTransform` components from `Transform` + `Parent`.
///
/// This intentionally lives in core because every feature crate can consume the
/// result without depending on each other.
pub fn propagate_global_transforms(engine: &mut Engine) {
    // Repair the derived index in case a low-level subsystem imported Parent
    // components directly.
    engine.rebuild_hierarchy_index();
    let entities: Vec<Entity> = engine.raw_world().entities().collect();
    let mut dirty_roots = engine.raw_world_mut().take_changed::<Transform>();
    dirty_roots.extend(engine.raw_world_mut().take_changed::<Parent>());
    dirty_roots.extend(
        engine
            .raw_world()
            .query::<TransformDirty>()
            .into_iter()
            .map(|(entity, _)| entity),
    );
    dirty_roots.sort_unstable();
    dirty_roots.dedup();
    if dirty_roots.is_empty() { return; }

    let mut locals = HashMap::<Entity, Transform>::new();
    let mut parents = HashMap::<Entity, Entity>::new();
    let mut children = HashMap::<Entity, Vec<Entity>>::new();
    for entity in &entities {
        if let Some(transform) = engine.raw_world().get::<Transform>(*entity) {
            locals.insert(*entity, transform.clone());
        }
        if let Some(parent) = engine.raw_world().get::<Parent>(*entity) {
            parents.insert(*entity, parent.0);
            children.entry(parent.0).or_default().push(*entity);
        }
    }

    let mut affected = HashSet::new();
    let mut queue = VecDeque::from(dirty_roots.clone());
    while let Some(entity) = queue.pop_front() {
        if !affected.insert(entity) { continue; }
        if let Some(descendants) = children.get(&entity) {
            queue.extend(descendants.iter().copied());
        }
    }

    let mut cache = HashMap::<Entity, GlobalTransform>::new();
    for entity in affected {
        if locals.contains_key(&entity) {
            let global = compute_entity(entity, &locals, &parents, &mut cache, &mut Vec::new());
            engine.raw_world_mut().insert(entity, global);
        }
    }
    for entity in dirty_roots {
        engine.raw_world_mut().remove::<TransformDirty>(entity);
    }
}

/// Legacy compatibility plugin. `Engine::new` installs transform propagation
/// in the standard schedule automatically.
pub struct HierarchyPlugin;

impl HierarchyPlugin {
    pub fn new() -> Self { Self }
}

impl Default for HierarchyPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for HierarchyPlugin {
    fn name(&self) -> &'static str { "core_hierarchy" }
    fn update_stage(&self) -> Stage { Stage::PostUpdate }

    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn Error>> { Ok(()) }

    fn update(&mut self, _engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        // Engine::new owns the scheduled propagation systems; avoid duplicate work.
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl Engine {
    /// Immediately refresh derived world transforms. Most apps can rely on the
    /// built-in post-update system; tools may call this after a batch edit that
    /// needs world-space results in the same function.
    pub fn sync_transforms(&mut self) { propagate_global_transforms(self); }
}
