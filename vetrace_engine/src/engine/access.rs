use super::Engine;
use crate::components::components::{
    BallJoint, KinematicBody, RevoluteJoint, RigidBody3D, StaticBody,
};
use crate::components::generated::{GeneratedComponent, GeneratedStorage};
use crate::ecs::{Component, Entity};
use crate::inspector::Inspectable;
use std::any::TypeId;

impl Engine {
    pub fn add_component<T: Component + Default + 'static>(&mut self, object_index: usize) {
        if let Some(entity) = self.core.find_entity_by_object_id(object_index as u32) {
            if !self.world.has::<T>(entity) {
                self.world.insert(entity, T::default());
            }
        }
    }

    pub fn has_component<T: Component + 'static>(&self, object_index: usize) -> bool {
        if let Some(entity) = self.core.find_entity_by_object_id(object_index as u32) {
            self.world.has::<T>(entity)
        } else {
            false
        }
    }

    pub fn remove_component<T: Component + 'static>(&mut self, object_index: usize) {
        if let Some(entity) = self.core.find_entity_by_object_id(object_index as u32) {
            self.remove_component_entity::<T>(entity);
        }
    }

    pub fn list_components(&self, object_index: usize) -> Vec<String> {
        let mut result = Vec::new();
        if let Some(entity) = self.core.find_entity_by_object_id(object_index as u32) {
            for (name, checker) in &self.component_checkers {
                if checker(&self.world, entity) {
                    result.push(name.clone());
                }
            }
        }
        result
    }

    pub fn add_component_entity<T: Component + Default + 'static>(&mut self, entity: Entity) {
        if !self.world.has::<T>(entity) {
            self.world.insert(entity, T::default());
        }
    }

    pub fn remove_component_entity<T: Component + 'static>(&mut self, entity: Entity) {
        let type_id = TypeId::of::<T>();
        if type_id == TypeId::of::<RigidBody3D>() {
            if let Some(rb) = self.world.get::<RigidBody3D>(entity) {
                if let Some(handle) = rb.handle {
                    self.physics.bodies.remove(
                        handle,
                        &mut self.physics.island_manager,
                        &mut self.physics.colliders,
                        &mut self.physics.joints,
                        &mut self.physics.multibody_joints,
                        true,
                    );
                }
            }
        } else if type_id == TypeId::of::<StaticBody>() {
            if let Some(sb) = self.world.get::<StaticBody>(entity) {
                if let Some(handle) = sb.handle {
                    self.physics.bodies.remove(
                        handle,
                        &mut self.physics.island_manager,
                        &mut self.physics.colliders,
                        &mut self.physics.joints,
                        &mut self.physics.multibody_joints,
                        true,
                    );
                }
            }
        } else if type_id == TypeId::of::<KinematicBody>() {
            if let Some(kb) = self.world.get::<KinematicBody>(entity) {
                if let Some(handle) = kb.handle {
                    self.physics.bodies.remove(
                        handle,
                        &mut self.physics.island_manager,
                        &mut self.physics.colliders,
                        &mut self.physics.joints,
                        &mut self.physics.multibody_joints,
                        true,
                    );
                }
            }
        } else if type_id == TypeId::of::<RevoluteJoint>() {
            if let Some(j) = self.world.get::<RevoluteJoint>(entity) {
                if let Some(handle) = j.handle {
                    self.physics.joints.remove(handle, true);
                }
            }
        } else if type_id == TypeId::of::<BallJoint>() {
            if let Some(j) = self.world.get::<BallJoint>(entity) {
                if let Some(handle) = j.handle {
                    self.physics.joints.remove(handle, true);
                }
            }
        }
        self.world.remove::<T>(entity);
    }
    pub fn get_component_mut_entity<T: Component + 'static>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        self.world.get_mut::<T>(entity)
    }

    pub fn get_component_mut<T: Component + 'static>(
        &mut self,
        object_index: usize,
    ) -> Option<&mut T> {
        if let Some(entity) = self.core.find_entity_by_object_id(object_index as u32) {
            return self.world.get_mut::<T>(entity);
        }
        None
    }

    pub fn access_component_mut<'a>(
        &'a mut self,
        entity: Entity,
        name: &str,
    ) -> Option<&'a mut dyn Inspectable> {
        if let Some(accessor) = self.component_accessors.get(name) {
            accessor(self, entity)
        } else {
            self.get_generated_component_mut(entity, name)
                .map(|c| c as &mut dyn Inspectable)
        }
    }

    pub fn list_components_entity(&self, entity: Entity) -> Vec<String> {
        let mut result = Vec::new();
        for (name, checker) in &self.component_checkers {
            if checker(&self.world, entity) {
                result.push(name.clone());
            }
        }
        result
    }

    pub fn add_generated_component(&mut self, entity: Entity, name: &str) {
        if let Some(spec) = self.generated_specs.get(name).cloned() {
            if !self.world.has::<GeneratedStorage>(entity) {
                self.world.insert(entity, GeneratedStorage::default());
            }
            if let Some(store) = self.world.get_mut::<GeneratedStorage>(entity) {
                store
                    .components
                    .entry(name.to_string())
                    .or_insert_with(|| spec.instance());
            }
        }
    }

    pub fn remove_generated_component(&mut self, entity: Entity, name: &str) {
        if let Some(store) = self.world.get_mut::<GeneratedStorage>(entity) {
            store.components.remove(name);
        }
    }

    pub fn get_generated_component_mut(
        &mut self,
        entity: Entity,
        name: &str,
    ) -> Option<&mut GeneratedComponent> {
        self.world
            .get_mut::<GeneratedStorage>(entity)
            .and_then(|s| s.components.get_mut(name))
    }

    pub fn has_generated_component(&self, entity: Entity, name: &str) -> bool {
        self.world
            .get::<GeneratedStorage>(entity)
            .map(|s| s.components.contains_key(name))
            .unwrap_or(false)
    }

    pub fn get_entity_name(&self, entity: Entity) -> Option<&str> {
        self.world
            .get::<crate::components::components::Metadata>(entity)
            .map(|m| m.name.as_str())
    }

    pub fn entity_has_tag(&self, entity: Entity, tag: &str) -> bool {
        self.world
            .get::<crate::components::components::Metadata>(entity)
            .map(|m| m.tags.iter().any(|t| t == tag))
            .unwrap_or(false)
    }

    pub fn remove_component_by_name(&mut self, entity: Entity, name: &str) {
        if let Some(rem) = self.component_removers.get(name).cloned() {
            rem(self, entity);
        } else {
            self.remove_generated_component(entity, name);
        }
    }

    pub fn delete_entity(&mut self, entity: Entity) {
        let obj_index = self
            .world
            .get::<crate::components::components::ObjectRef>(entity)
            .map(|r| r.id as usize);
        let comps = self.list_components_entity(entity);
        for c in comps {
            self.remove_component_by_name(entity, &c);
        }
        self.world
            .remove::<crate::components::components::Metadata>(entity);
        self.world
            .remove::<crate::components::components::ObjectRef>(entity);
        self.world.delete_entity(entity);
        self.core.object_entity_map.retain(|_, &mut e| e != entity);
        if let Some(index) = obj_index {
            self.scene.remove_object(index);
            #[cfg(feature = "wgpu")]
            self.invalidate_material_cache();
            let mut new_map = std::collections::HashMap::new();
            for (id, ent) in self.core.object_entity_map.iter_mut() {
                let mut new_id = *id;
                if *id > index as u32 {
                    new_id -= 1;
                }
                if let Some(obj_ref) = self
                    .world
                    .get_mut::<crate::components::components::ObjectRef>(*ent)
                {
                    if obj_ref.id > index as u32 {
                        obj_ref.id -= 1;
                    }
                }
                new_map.insert(new_id, *ent);
            }
            self.core.object_entity_map = new_map;
        }
    }
}
