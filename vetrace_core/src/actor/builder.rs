use crate::components::builtins::{ActorId, Metadata, Name, Transform};
use crate::{Bundle, Component, Engine};

use super::{Actor, ActorError};

/// Fluent actor construction. Dropping an unfinished or failed builder rolls
/// back the partially-created actor.
pub struct ActorBuilder<'a> {
    engine: &'a mut Engine,
    actor: Actor,
    error: Option<ActorError>,
    committed: bool,
}

impl<'a> ActorBuilder<'a> {
    pub(crate) fn new(engine: &'a mut Engine, name: String) -> Self {
        let entity = engine.world_mut().spawn();
        engine.world_mut().insert(entity, ActorId::new());
        engine.world_mut().insert(entity, Name(name));
        let actor = Actor::from_entity(entity);
        let mut builder = Self { engine, actor, error: None, committed: false };
        builder = builder.with(Transform::default()).with(Metadata::default());
        builder
    }

    /// Replace the automatically generated persistent ID, typically while
    /// importing authored data.
    pub fn id(mut self, id: ActorId) -> Self {
        if self.error.is_none() {
            if let Err(error) = self.actor.set_id(&mut *self.engine, id) {
                self.error = Some(error);
            }
        }
        self
    }

    /// Add one component while preserving Actor-managed component invariants.
    /// Any failure is retained and returned by `try_build`.
    pub fn with<T: Component>(mut self, component: T) -> Self {
        if self.error.is_none() {
            if let Err(error) = self.actor.insert(&mut *self.engine, component) {
                self.error = Some(error);
            }
        }
        self
    }

    /// Add a reusable component bundle.
    pub fn bundle<B: Bundle>(mut self, bundle: B) -> Self {
        if self.error.is_none() {
            if let Err(error) = bundle.insert(self.actor, &mut *self.engine) {
                self.error = Some(error);
            }
        }
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        if self.error.is_none() {
            if let Err(error) = self.actor.add_tag(&mut *self.engine, tag) {
                self.error = Some(error);
            }
        }
        self
    }

    pub fn source(mut self, source: impl Into<String>) -> Self {
        if self.error.is_none() {
            if let Err(error) = self.actor.set_source(&mut *self.engine, Some(source.into())) {
                self.error = Some(error);
            }
        }
        self
    }

    /// Parent this actor. A hierarchy failure immediately returns the error and
    /// dropping the builder removes the incomplete actor.
    pub fn child_of(self, parent: Actor) -> Result<Self, ActorError> {
        self.actor.set_parent(&mut *self.engine, parent)?;
        Ok(self)
    }

    /// Build with explicit error handling. Failed builds are rolled back.
    pub fn try_build(mut self) -> Result<Actor, ActorError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        self.committed = true;
        Ok(self.actor)
    }

    /// Convenience build for statically-known component sets. Use `try_build`
    /// when parent/component validity is data-driven.
    pub fn build(self) -> Actor {
        self.try_build().expect("ActorBuilder failed; use try_build() to handle data-driven construction errors")
    }
}

impl Drop for ActorBuilder<'_> {
    fn drop(&mut self) {
        if !self.committed && self.actor.is_alive(self.engine) {
            for child in self.actor.children(self.engine) {
                let _ = child.clear_parent(self.engine);
            }
            self.engine.remove_from_hierarchy_index(self.actor.entity());
            self.engine.world_mut().despawn(self.actor.entity());
        }
    }
}
