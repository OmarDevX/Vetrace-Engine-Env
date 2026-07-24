use std::any::{Any, TypeId};
use std::collections::BTreeSet;

use glam::{Quat, Vec3};

use crate::components::builtins::{ActorId, Children, GlobalTransform, Metadata, Name, Parent, Transform, TransformDirty};
use crate::{Component, Engine, Entity};

use super::ActorError;

/// Lightweight high-level handle to an ECS object.
///
/// `Actor` owns no components. The ECS remains the single source of component
/// data, while this generational handle enforces naming, transform, hierarchy,
/// identity, and destruction invariants.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Actor {
    entity: Entity,
}

#[derive(Clone, Copy, Debug)]
pub struct ActorDestroyed {
    pub actor: Actor,
    pub id: Option<ActorId>,
}

impl Actor {
    pub const INVALID: Self = Self { entity: Entity::INVALID };

    /// Wrap a low-level runtime handle. Prefer `Engine::actor` when the handle
    /// may be stale and needs validation.
    pub const fn from_entity(entity: Entity) -> Self { Self { entity } }
    pub const fn entity(self) -> Entity { self.entity }
    pub const fn raw(self) -> u64 { self.entity.raw() }

    pub fn is_alive(self, engine: &Engine) -> bool { engine.world_ref().is_alive(self.entity) }

    pub fn id(self, engine: &Engine) -> Option<ActorId> {
        self.get_component::<ActorId>(engine).copied()
    }

    /// Explicitly assign persistent identity, primarily for scene/save import.
    /// Normal runtime spawning creates a fresh ActorId automatically.
    pub fn set_id(self, engine: &mut Engine, id: ActorId) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        if engine.find_actor_by_id(id).is_some_and(|existing| existing != self) {
            return Err(ActorError::DuplicateActorId(id));
        }
        engine.world_mut().insert(self.entity, id);
        Ok(())
    }

    pub fn name<'a>(self, engine: &'a Engine) -> Option<&'a str> {
        engine.world_ref().get::<Name>(self.entity).map(|name| name.0.as_str())
    }

    pub fn set_name(self, engine: &mut Engine, name: impl Into<String>) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        let name = name.into();
        if let Some(existing) = engine.world_mut().get_mut::<Name>(self.entity) {
            existing.0 = name;
        } else {
            engine.world_mut().insert(self.entity, Name(name));
        }
        Ok(())
    }

    pub fn transform<'a>(self, engine: &'a Engine) -> Option<&'a Transform> {
        engine.world_ref().get::<Transform>(self.entity)
    }

    pub fn transform_mut<'a>(self, engine: &'a mut Engine) -> Option<&'a mut Transform> {
        if !self.is_alive(engine) { return None; }
        mark_transform_dirty(engine, self.entity);
        engine.world_mut().get_mut::<Transform>(self.entity)
    }

    pub fn global_transform<'a>(self, engine: &'a Engine) -> Option<&'a GlobalTransform> {
        engine.world_ref().get::<GlobalTransform>(self.entity)
    }

    pub fn set_transform(self, engine: &mut Engine, transform: Transform) -> Result<(), ActorError> {
        self.insert(engine, transform)
    }

    pub fn set_position(self, engine: &mut Engine, position: Vec3) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        mark_transform_dirty(engine, self.entity);
        let transform = engine.world_mut().get_mut::<Transform>(self.entity).ok_or(ActorError::ManagedComponent("Transform"))?;
        transform.translation = position;
        Ok(())
    }

    pub fn translate(self, engine: &mut Engine, offset: Vec3) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        mark_transform_dirty(engine, self.entity);
        let transform = engine.world_mut().get_mut::<Transform>(self.entity).ok_or(ActorError::ManagedComponent("Transform"))?;
        transform.translation += offset;
        Ok(())
    }

    pub fn set_rotation(self, engine: &mut Engine, rotation: Quat) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        mark_transform_dirty(engine, self.entity);
        let transform = engine.world_mut().get_mut::<Transform>(self.entity).ok_or(ActorError::ManagedComponent("Transform"))?;
        transform.rotation = rotation;
        Ok(())
    }

    pub fn set_scale(self, engine: &mut Engine, scale: Vec3) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        mark_transform_dirty(engine, self.entity);
        let transform = engine.world_mut().get_mut::<Transform>(self.entity).ok_or(ActorError::ManagedComponent("Transform"))?;
        transform.scale = scale;
        Ok(())
    }

    pub fn look_at(self, engine: &mut Engine, target: Vec3, _up: Vec3) -> Result<(), ActorError> {
        let position = self.transform(engine).map(|transform| transform.translation).ok_or(ActorError::ManagedComponent("Transform"))?;
        let forward = (target - position).normalize_or_zero();
        if forward.length_squared() <= f32::EPSILON { return Ok(()); }
        self.set_rotation(engine, Quat::from_rotation_arc(Vec3::NEG_Z, forward))
    }

    pub fn get_component<'a, T: Component>(self, engine: &'a Engine) -> Option<&'a T> {
        engine.world_ref().get::<T>(self.entity)
    }

    pub fn get_component_mut<'a, T: Component>(self, engine: &'a mut Engine) -> Option<&'a mut T> {
        let type_id = TypeId::of::<T>();
        if is_actor_managed_mutation(type_id) { return None; }
        if type_id == TypeId::of::<Transform>() { mark_transform_dirty(engine, self.entity); }
        engine.world_mut().get_mut::<T>(self.entity)
    }

    pub fn component<'a, T: Component>(self, engine: &'a Engine) -> Option<&'a T> { self.get_component::<T>(engine) }
    pub fn component_mut<'a, T: Component>(self, engine: &'a mut Engine) -> Option<&'a mut T> {
        self.get_component_mut::<T>(engine)
    }

    pub fn has<T: Component>(self, engine: &Engine) -> bool { engine.world_ref().has::<T>(self.entity) }

    pub fn insert<T: Component>(self, engine: &mut Engine, component: T) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        if let Some(parent) = component_as_parent(&component) {
            return self.set_parent(engine, Actor::from_entity(parent.0));
        }
        let type_id = TypeId::of::<T>();
        if type_id == TypeId::of::<Children>() {
            return Err(ActorError::ManagedComponent("Children"));
        }
        if type_id == TypeId::of::<GlobalTransform>() {
            return Err(ActorError::ManagedComponent("GlobalTransform"));
        }
        if type_id == TypeId::of::<TransformDirty>() {
            return Err(ActorError::ManagedComponent("TransformDirty"));
        }
        if component_as_actor_id(&component).is_some() {
            return Err(ActorError::ManagedComponent("ActorId; use Actor::set_id"));
        }
        insert_actor_component(engine, self.entity, component);
        Ok(())
    }

    pub fn remove<T: Component>(self, engine: &mut Engine) -> Option<T> {
        if !self.is_alive(engine) { return None; }
        let type_id = TypeId::of::<T>();
        if matches_managed_removal(type_id) { return None; }
        if type_id != TypeId::of::<Parent>() {
            return engine.world_mut().remove::<T>(self.entity);
        }

        let parent = engine.world_mut().remove::<Parent>(self.entity);
        engine.update_hierarchy_index(self.entity, None);
        mark_transform_dirty(engine, self.entity);
        parent.and_then(|parent| {
            (Box::new(parent) as Box<dyn Any>).downcast::<T>().ok().map(|component| *component)
        })
    }

    pub fn metadata<'a>(self, engine: &'a Engine) -> Option<&'a Metadata> {
        engine.world_ref().get::<Metadata>(self.entity)
    }

    pub fn has_tag(self, engine: &Engine, tag: &str) -> bool {
        self.metadata(engine).map(|metadata| metadata.tags.iter().any(|candidate| candidate == tag)).unwrap_or(false)
    }

    pub fn add_tag(self, engine: &mut Engine, tag: impl Into<String>) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        let tag = tag.into();
        if let Some(metadata) = engine.world_mut().get_mut::<Metadata>(self.entity) {
            if !metadata.tags.iter().any(|candidate| candidate == &tag) { metadata.tags.push(tag); }
        } else {
            engine.world_mut().insert(self.entity, Metadata { tags: vec![tag], source: None });
        }
        Ok(())
    }

    pub fn remove_tag(self, engine: &mut Engine, tag: &str) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        if let Some(metadata) = engine.world_mut().get_mut::<Metadata>(self.entity) {
            metadata.tags.retain(|candidate| candidate != tag);
        }
        Ok(())
    }

    pub fn source<'a>(self, engine: &'a Engine) -> Option<&'a str> {
        self.metadata(engine).and_then(|metadata| metadata.source.as_deref())
    }

    pub fn set_source(self, engine: &mut Engine, source: Option<String>) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        if let Some(metadata) = engine.world_mut().get_mut::<Metadata>(self.entity) {
            metadata.source = source;
        } else {
            engine.world_mut().insert(self.entity, Metadata { tags: Vec::new(), source });
        }
        Ok(())
    }

    /// `Parent` is the serialized source of truth; `Hierarchy` is its derived
    /// traversal index.
    pub fn parent(self, engine: &Engine) -> Option<Actor> {
        engine
            .hierarchy()
            .and_then(|hierarchy| hierarchy.parent_of(self))
            .or_else(|| engine.world_ref().get::<Parent>(self.entity).map(|parent| Actor::from_entity(parent.0)))
            .filter(|parent| parent.is_alive(engine))
    }

    pub fn children(self, engine: &Engine) -> Vec<Actor> {
        if let Some(hierarchy) = engine.hierarchy() {
            return hierarchy.children_of(self).filter(|child| child.is_alive(engine)).collect();
        }
        engine
            .world_ref()
            .query::<Parent>()
            .into_iter()
            .filter_map(|(entity, parent)| (parent.0 == self.entity).then_some(Actor::from_entity(entity)))
            .collect()
    }

    pub fn set_parent(self, engine: &mut Engine, parent: Actor) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        parent.ensure_alive(engine).map_err(|_| ActorError::DeadParent(parent.entity))?;
        if self == parent { return Err(ActorError::CannotParentToSelf(self.entity)); }

        let mut cursor = Some(parent.entity);
        let mut visited = BTreeSet::new();
        while let Some(entity) = cursor {
            if entity == self.entity || !visited.insert(entity) {
                return Err(ActorError::HierarchyCycle { actor: self.entity, parent: parent.entity });
            }
            cursor = engine.world_ref().get::<Parent>(entity).map(|parent| parent.0);
        }

        engine.world_mut().insert(self.entity, Parent(parent.entity));
        engine.update_hierarchy_index(self.entity, Some(parent.entity));
        mark_transform_tree_dirty(engine, self);
        refresh_actor_global_transform(engine, self.entity);
        Ok(())
    }

    pub fn add_child(self, engine: &mut Engine, child: Actor) -> Result<(), ActorError> { child.set_parent(engine, self) }

    pub fn clear_parent(self, engine: &mut Engine) -> Result<(), ActorError> {
        self.ensure_alive(engine)?;
        engine.world_mut().remove::<Parent>(self.entity);
        engine.update_hierarchy_index(self.entity, None);
        mark_transform_tree_dirty(engine, self);
        refresh_actor_global_transform(engine, self.entity);
        Ok(())
    }

    /// Destroy this actor and its complete child hierarchy.
    pub fn despawn(self, engine: &mut Engine) -> bool {
        if !self.is_alive(engine) { return false; }
        let mut visited = BTreeSet::new();
        let mut post_order = Vec::new();
        collect_hierarchy_post_order(engine, self, &mut visited, &mut post_order);
        for actor in post_order {
            let event = ActorDestroyed { actor, id: actor.id(engine) };
            engine.send_event(event);
            engine.remove_from_hierarchy_index(actor.entity);
            engine.world_mut().despawn(actor.entity);
        }
        true
    }

    /// Destroy only this actor. Direct children survive as roots.
    pub fn despawn_only(self, engine: &mut Engine) -> bool {
        if !self.is_alive(engine) { return false; }
        for child in self.children(engine) { let _ = child.clear_parent(engine); }
        let event = ActorDestroyed { actor: self, id: self.id(engine) };
        engine.send_event(event);
        engine.remove_from_hierarchy_index(self.entity);
        engine.world_mut().despawn(self.entity)
    }

    pub(crate) fn ensure_alive(self, engine: &Engine) -> Result<(), ActorError> {
        if self.is_alive(engine) { Ok(()) } else { Err(ActorError::DeadActor(self.entity)) }
    }
}

pub(crate) fn is_actor_managed_mutation(type_id: TypeId) -> bool {
    type_id == TypeId::of::<ActorId>()
        || type_id == TypeId::of::<Parent>()
        || type_id == TypeId::of::<Children>()
        || type_id == TypeId::of::<GlobalTransform>()
        || type_id == TypeId::of::<TransformDirty>()
}

fn matches_managed_removal(type_id: TypeId) -> bool {
    type_id == TypeId::of::<ActorId>()
        || type_id == TypeId::of::<Children>()
        || type_id == TypeId::of::<GlobalTransform>()
        || type_id == TypeId::of::<TransformDirty>()
}

pub(crate) fn component_as_parent<T: Component>(component: &T) -> Option<Parent> {
    (component as &dyn Any).downcast_ref::<Parent>().copied()
}

pub(crate) fn component_as_actor_id<T: Component>(component: &T) -> Option<ActorId> {
    (component as &dyn Any).downcast_ref::<ActorId>().copied()
}

pub(crate) fn insert_actor_component<T: Component>(engine: &mut Engine, entity: Entity, component: T) {
    engine.world_mut().insert(entity, component);
    if TypeId::of::<T>() == TypeId::of::<Transform>() {
        mark_transform_dirty(engine, entity);
        refresh_actor_global_transform(engine, entity);
    }
}

pub(crate) fn mark_transform_dirty(engine: &mut Engine, entity: Entity) {
    if engine.world_ref().is_alive(entity) && !engine.world_ref().has::<TransformDirty>(entity) {
        engine.world_mut().insert(entity, TransformDirty);
    }
}

fn mark_transform_tree_dirty(engine: &mut Engine, root: Actor) {
    let mut stack = vec![root];
    let mut visited = BTreeSet::new();
    while let Some(actor) = stack.pop() {
        if !visited.insert(actor.entity) { continue; }
        mark_transform_dirty(engine, actor.entity);
        stack.extend(actor.children(engine));
    }
}

fn combine_global_transform(parent: &GlobalTransform, local: &Transform) -> GlobalTransform {
    GlobalTransform {
        translation: parent.translation + parent.rotation * (local.translation * parent.scale),
        rotation: parent.rotation * local.rotation,
        scale: parent.scale * local.scale,
    }
}

fn refresh_actor_global_transform(engine: &mut Engine, entity: Entity) {
    let Some(local) = engine.world_ref().get::<Transform>(entity).cloned() else { return; };
    let global = engine
        .world_ref()
        .get::<Parent>(entity)
        .and_then(|parent| {
            engine
                .world_ref()
                .get::<GlobalTransform>(parent.0)
                .cloned()
                .or_else(|| engine.world_ref().get::<Transform>(parent.0).map(GlobalTransform::from))
        })
        .map(|parent| combine_global_transform(&parent, &local))
        .unwrap_or_else(|| GlobalTransform::from(&local));
    engine.world_mut().insert(entity, global);
}

fn collect_hierarchy_post_order(
    engine: &Engine,
    actor: Actor,
    visited: &mut BTreeSet<Entity>,
    post_order: &mut Vec<Actor>,
) {
    if !actor.is_alive(engine) || !visited.insert(actor.entity) { return; }
    for child in actor.children(engine) { collect_hierarchy_post_order(engine, child, visited, post_order); }
    post_order.push(actor);
}

impl From<Entity> for Actor {
    fn from(entity: Entity) -> Self { Self::from_entity(entity) }
}

impl From<Actor> for Entity {
    fn from(actor: Actor) -> Self { actor.entity }
}
