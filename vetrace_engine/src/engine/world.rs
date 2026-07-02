use super::{Engine, Actor};
use crate::ecs::Entity;

/// High-level scene access wrapper inspired by C++ `World`.
///
/// Returned from [`Engine::world`], this struct lets you spawn and
/// query [`Actor`]s without dealing with the engine directly.
pub struct World<'a> {
    pub(crate) engine: &'a mut Engine,
}

impl<'a> World<'a> {
    pub(crate) fn new(engine: &'a mut Engine) -> Self {
        Self { engine }
    }

    /// Spawn an empty actor with a name.
    pub fn spawn_actor(&'a mut self, name: &str) -> Actor<'a> {
        let entity = self.engine.spawn_empty(name);
        Actor::new(self.engine, entity)
    }

    /// Wrap an existing entity as an [`Actor`].
    pub fn actor_from_entity(&'a mut self, entity: Entity) -> Actor<'a> {
        Actor::new(self.engine, entity)
    }

    /// Get an [`Actor`] by object index if it exists.
    pub fn actor_from_object(&'a mut self, index: usize) -> Option<Actor<'a>> {
        Actor::from_object_index(self.engine, index)
    }

    /// Load a scene file using the underlying engine utilities.
    pub fn load_scene(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.engine.load_scene_from_file(path)
    }
}
