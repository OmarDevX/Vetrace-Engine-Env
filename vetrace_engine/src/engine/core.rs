// Updated `core.rs` for your engine with component and entity helpers

use std::collections::HashMap;

use crate::ecs::{Component, Entity, World};

/// Core ECS data shared across the engine.
pub struct EngineCore {
    /// The world containing all entities and components.
    pub world: World,
    /// Mapping from scene object IDs to ECS entities.
    pub object_entity_map: HashMap<u32, Entity>,
}

impl EngineCore {
    /// Create a new [`EngineCore`] with an empty [`World`].
    pub fn new() -> Self {
        Self {
            world: World::new(),
            object_entity_map: HashMap::new(),
        }
    }

    /// Add a default instance of the component `T` to the specified object.
    ///
    /// The object index is resolved to an entity and the component is inserted
    /// if it does not already exist.
    pub fn add_component<T: Component + Default + 'static>(&mut self, object_index: usize) {
        if let Some(entity) = self.find_entity_by_object_id(object_index as u32) {
            if !self.world.has::<T>(entity) {
                self.world.insert(entity, T::default());
            }
        }
    }



    /// Check whether the object at `object_index` contains a component of type `T`.
    pub fn has_component<T: Component + 'static>(&self, object_index: usize) -> bool {
        if let Some(entity) = self.find_entity_by_object_id(object_index as u32) {
            return self.world.has::<T>(entity);
        }
        false
    }

    /// Remove the component `T` from the specified object if present.
    pub fn remove_component<T: Component + 'static>(&mut self, object_index: usize) {
        if let Some(entity) = self.find_entity_by_object_id(object_index as u32) {
            self.world.remove::<T>(entity);
        }
    }

    /// Map an object ID from the scene to a newly created entity.
    pub fn register_object_entity(&mut self, object_id: u32, entity: Entity) {
        self.object_entity_map.insert(object_id, entity);
    }

    /// Retrieve the entity associated with a given object ID.
    pub fn find_entity_by_object_id(&self, object_id: u32) -> Option<Entity> {
        self.object_entity_map.get(&object_id).copied()
    }

    /// Get the object ID associated with an entity.
    pub fn find_object_id_by_entity(&self, entity: Entity) -> Option<u32> {
        for (id, ent) in self.object_entity_map.iter() {
            if *ent == entity {
                return Some(*id);
            }
        }
        None
    }
}
