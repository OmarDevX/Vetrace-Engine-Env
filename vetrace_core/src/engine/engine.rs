use std::time::Duration;

use crate::actor::{Actor, ActorBuilder};
use crate::actor::is_actor_managed_mutation;
use crate::backends::ProfilerBackend;
use crate::commands::DeferredCommand;
use crate::components::builtins::{ActorId, Children, GlobalTransform, Metadata, Name, ObjectRef, Parent, Timer, Transform, TransformDirty};
use crate::ecs::{Component, Entity, World};
use crate::engine::component_registry::ComponentManager;
use crate::input::InputState;
use crate::hierarchy::Hierarchy;
use crate::query::{MutQuery, MutQueryWith, Query, QuerySpec};
use crate::resources::Resources;
use crate::scene::{EntityDef, SceneDef};
use crate::schedule::{FixedTime, Schedule};

/// Generic engine state. Component storage is intentionally private so game
/// code cannot bypass `Actor` invariants accidentally.
pub struct Engine {
    world: World,
    resources: Resources,
    running: bool,
    active_scene: Option<SceneDef>,
    pub(crate) pending_commands: Vec<DeferredCommand>,
}

impl Default for Engine {
    fn default() -> Self { Self::new() }
}

impl Engine {
    pub fn new() -> Self {
        let mut engine = Self {
            world: World::new(),
            resources: Resources::new(),
            running: true,
            active_scene: None,
            pending_commands: Vec::new(),
        };
        engine.insert_resource(ComponentManager::new());
        engine.insert_resource(InputState::new());
        engine.insert_resource(Schedule::default());
        engine.insert_resource(FixedTime::default());
        engine.insert_resource(Hierarchy::default());
        if let Some(registry) = engine.get_resource_mut::<ComponentManager>() {
            registry.register_serializable_readonly::<ActorId>("vetrace.core.actor_id", "Actor ID");
            registry.register_reflected_named::<Transform>("vetrace.core.transform", "Transform", "Core");
            registry.register_serializable_readonly::<GlobalTransform>("vetrace.core.global_transform", "Global Transform");
            registry.register_reflected_named::<Name>("vetrace.core.name", "Name", "Core");
            registry.register_named::<Parent>("vetrace.core.parent", "Parent");
            #[allow(deprecated)]
            registry.register_named::<Children>("vetrace.core.children_compat", "Children (Compatibility)");
            registry.register_reflected_named::<ObjectRef>("vetrace.core.object_ref", "Object Reference", "Core");
            registry.register_reflected_named::<Metadata>("vetrace.core.metadata", "Metadata", "Core");
            registry.register_reflected_named::<Timer>("vetrace.core.timer", "Timer", "Core");
            registry.register_named::<TransformDirty>("vetrace.core.transform_dirty", "Transform Dirty");
            let _ = registry.set_removable("vetrace.core.transform", false);
            let _ = registry.set_removable("vetrace.core.name", false);
            let _ = registry.register_alias("vetrace.core.transform", "transform");
            let _ = registry.register_alias("vetrace.core.name", "name");
            let _ = registry.register_alias("vetrace.core.metadata", "metadata");
        }

        // Core lifecycle systems are always available. Applications no longer
        // need to remember optional plugins just to advance timers or keep
        // GlobalTransform synchronized after simulation.
        engine.add_system(crate::Stage::FixedUpdate, "core.tick_timers", |engine, dt| {
            crate::systems::tick_timers(engine, dt);
        });
        engine.add_system(crate::Stage::PostUpdate, "core.propagate_global_transforms", |engine, _dt| {
            crate::systems::propagate_global_transforms(engine);
        });
        // Catch transforms written by post-update plugins such as animation
        // before render extraction. This is normally a cheap no-op because the
        // propagation system consumes its change set.
        engine.add_system(crate::Stage::RenderExtract, "core.propagate_global_transforms_late", |engine, _dt| {
            crate::systems::propagate_global_transforms(engine);
        });
        engine
    }

    pub(crate) fn world_ref(&self) -> &World { &self.world }
    pub(crate) fn world_mut(&mut self) -> &mut World { &mut self.world }

    /// Escape hatch for subsystem internals that need raw ECS-wide access.
    /// Game code should use `Actor`, `Query`, and `Commands` instead.
    #[doc(hidden)]
    pub fn raw_world(&self) -> &World { &self.world }

    /// Mutable low-level escape hatch for render/physics/editor internals.
    #[doc(hidden)]
    pub fn raw_world_mut(&mut self) -> &mut World { &mut self.world }

    pub fn insert_resource<T: 'static>(&mut self, value: T) -> Option<T> { self.resources.insert(value) }
    pub fn get_resource<T: 'static>(&self) -> Option<&T> { self.resources.get::<T>() }
    pub fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> { self.resources.get_mut::<T>() }
    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> { self.resources.remove::<T>() }
    pub fn contains_resource<T: 'static>(&self) -> bool { self.resources.contains::<T>() }

    /// Temporarily removes a resource while an operation receives both the
    /// resource and the complete engine. This avoids aliasing `&mut Engine` with
    /// a mutable reference borrowed from the engine's resource store.
    ///
    /// The resource is restored before returning and also restored if the
    /// operation panics. A same-typed resource inserted by the operation is
    /// replaced by the original resource when the scope ends.
    pub fn with_resource_removed<T: 'static, R>(
        &mut self,
        operation: impl FnOnce(&mut T, &mut Engine) -> R,
    ) -> Option<R> {
        let mut resource = self.remove_resource::<T>()?;
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            operation(&mut resource, self)
        }));
        self.insert_resource(resource);
        match outcome {
            Ok(result) => Some(result),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub fn profile_begin_frame(&mut self) {
        if let Some(profiler) = self.get_resource_mut::<Box<dyn ProfilerBackend>>() { profiler.begin_frame(); }
    }
    pub fn profile_end_frame(&mut self) {
        if let Some(profiler) = self.get_resource_mut::<Box<dyn ProfilerBackend>>() { profiler.end_frame(); }
    }
    pub fn profile_record_timing(&mut self, name: &str, duration: Duration) {
        if let Some(profiler) = self.get_resource_mut::<Box<dyn ProfilerBackend>>() { profiler.record_timing(name, duration); }
    }
    pub fn profile_record_counter(&mut self, name: &str, value: f64, unit: &'static str) {
        if let Some(profiler) = self.get_resource_mut::<Box<dyn ProfilerBackend>>() { profiler.record_counter(name, value, unit); }
    }

    pub fn spawn_actor(&mut self, name: impl Into<String>) -> ActorBuilder<'_> {
        ActorBuilder::new(self, name.into())
    }

    pub fn actor(&self, entity: Entity) -> Option<Actor> {
        self.world.is_alive(entity).then_some(Actor::from_entity(entity))
    }

    pub fn actors(&self) -> Vec<Actor> { self.world.entities().map(Actor::from_entity).collect() }

    pub fn actors_with<T: Component>(&self) -> Vec<(Actor, &T)> {
        self.world.query::<T>().into_iter().map(|(entity, component)| (Actor::from_entity(entity), component)).collect()
    }

    #[deprecated(note = "use Engine::query_mut; actors_with_mut allocates and is kept only for compatibility")]
    pub fn actors_with_mut<T: Component>(&mut self) -> Vec<(Actor, &mut T)> {
        assert!(!is_actor_managed_mutation(std::any::TypeId::of::<T>()), "managed Actor components require their dedicated API");
        self.world.query_mut::<T>().into_iter().map(|(entity, component)| (Actor::from_entity(entity), component)).collect()
    }

    pub fn query<'world, Q>(&'world self) -> Query<'world, Q>
    where
        Q: QuerySpec<'world>,
    {
        Query::new(self)
    }

    pub fn query_mut<A: Component>(&mut self) -> MutQuery<'_, A> {
        MutQuery::new(self)
    }

    pub fn query_mut_with<A: Component, B: Component>(&mut self) -> MutQueryWith<'_, A, B> {
        MutQueryWith::new(self)
    }

    pub fn find_actor_by_name(&self, name: &str) -> Option<Actor> {
        self.actors_with::<Name>().into_iter().find_map(|(actor, actor_name)| (actor_name.0 == name).then_some(actor))
    }

    pub fn find_actor_by_id(&self, id: ActorId) -> Option<Actor> {
        self.actors_with::<ActorId>().into_iter().find_map(|(actor, actor_id)| (*actor_id == id).then_some(actor))
    }

    pub fn load_scene_def(&mut self, scene: SceneDef) { self.active_scene = Some(scene); }
    pub fn active_scene(&self) -> Option<&SceneDef> { self.active_scene.as_ref() }
    pub fn take_active_scene(&mut self) -> Option<SceneDef> { self.active_scene.take() }

    pub fn spawn_from_def(&mut self, def: &EntityDef) -> Actor {
        let actor = self.spawn_actor(def.name.clone().unwrap_or_else(|| "Actor".to_string())).build();
        if let Some(id) = def.id { let _ = actor.set_id(self, id); }
        actor
    }

    pub fn clear_world(&mut self) {
        self.pending_commands.clear();
        self.world.clear();
        self.insert_resource(Hierarchy::default());
    }

    pub fn stop(&mut self) { self.running = false; }
    pub fn is_running(&self) -> bool { self.running }

    // Temporary compatibility surface. These methods are deliberately
    // deprecated so new gameplay does not create a second API beside Actor.
    #[deprecated(note = "use Engine::spawn_actor")]
    pub fn spawn(&mut self) -> Entity { self.spawn_actor("Actor").build().entity() }

    #[deprecated(note = "use Engine::spawn_actor")]
    pub fn spawn_named(&mut self, name: impl Into<String>) -> Entity { self.spawn_actor(name).build().entity() }

    #[deprecated(note = "use Actor::insert")]
    pub fn insert_component<T: Component>(&mut self, entity: Entity, component: T) {
        if let Some(actor) = self.actor(entity) { let _ = actor.insert(self, component); }
    }

    #[deprecated(note = "use Engine::find_actor_by_name")]
    pub fn find_entity_by_name(&self, name: &str) -> Option<Entity> { self.find_actor_by_name(name).map(Actor::entity) }

    #[deprecated(note = "use Actor::name")]
    pub fn get_entity_name(&self, entity: Entity) -> Option<&str> { self.actor(entity)?.name(self) }

    #[deprecated(note = "use Actor::set_name")]
    pub fn set_entity_name(&mut self, entity: Entity, name: impl Into<String>) {
        if let Some(actor) = self.actor(entity) { let _ = actor.set_name(self, name); }
    }

    #[deprecated(note = "use Actor::has_tag")]
    pub fn entity_has_tag(&self, entity: Entity, tag: &str) -> bool {
        self.actor(entity).map(|actor| actor.has_tag(self, tag)).unwrap_or(false)
    }

    #[deprecated(note = "use Actor::add_tag")]
    pub fn add_entity_tag(&mut self, entity: Entity, tag: impl Into<String>) {
        if let Some(actor) = self.actor(entity) { let _ = actor.add_tag(self, tag); }
    }

    #[deprecated(note = "use Actor::remove_tag")]
    pub fn remove_entity_tag(&mut self, entity: Entity, tag: &str) {
        if let Some(actor) = self.actor(entity) { let _ = actor.remove_tag(self, tag); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct TestResource(u32);

    #[test]
    fn temporarily_removed_resource_is_restored_after_success() {
        let mut engine = Engine::new();
        engine.insert_resource(TestResource(4));

        let result = engine.with_resource_removed::<TestResource, _>(|resource, engine| {
            resource.0 += 3;
            assert!(!engine.contains_resource::<TestResource>());
            resource.0 * 2
        });

        assert_eq!(result, Some(14));
        assert_eq!(engine.get_resource::<TestResource>(), Some(&TestResource(7)));
    }

    #[test]
    fn temporarily_removed_resource_is_restored_after_panic() {
        let mut engine = Engine::new();
        engine.insert_resource(TestResource(9));

        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.with_resource_removed::<TestResource, ()>(|resource, _engine| {
                resource.0 = 12;
                panic!("test panic");
            });
        }));

        assert!(panic.is_err());
        assert_eq!(engine.get_resource::<TestResource>(), Some(&TestResource(12)));
    }
}
