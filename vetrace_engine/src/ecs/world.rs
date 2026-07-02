use crate::inspector::InspectableComponent;
use std::any::{Any, TypeId};

use super::component::Component;
use super::entity::Entity;
use ahash::{HashMap, HashMapExt};

pub struct World {
    next_id: u32,
    components: HashMap<TypeId, Box<dyn Any>>,
    pub entities: Vec<Entity>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            components: HashMap::new(),
            entities: Vec::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let entity = Entity(self.next_id);
        self.next_id += 1;
        self.entities.push(entity);
        entity
    }

    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        let storage = self
            .components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<Entity, T>::new()))
            .downcast_mut::<HashMap<Entity, T>>()
            .unwrap();

        storage.insert(entity, component);
    }

    /// Get a component from an entity
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let storage = self
            .components
            .get(&TypeId::of::<T>())?
            .downcast_ref::<HashMap<Entity, T>>()?;
        storage.get(&entity)
    }

    /// Get a mutable component from an entity
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let storage = self
            .components
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<Entity, T>>()?;
        storage.get_mut(&entity)
    }

    /// Check if an entity has a component
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        if let Some(storage) = self.components.get(&TypeId::of::<T>()) {
            if let Some(storage) = storage.downcast_ref::<HashMap<Entity, T>>() {
                return storage.contains_key(&entity);
            }
        }
        false
    }

    /// Remove a component from an entity
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let storage = self
            .components
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<HashMap<Entity, T>>()?;
        storage.remove(&entity)
    }

    /// Get all entities
    pub fn entities(&self) -> &Vec<Entity> {
        &self.entities
    }

    /// Query for all entities that have component `T`, returning mutable access
    /// to each component along with the entity id.
    pub fn query_mut<T: Component>(&mut self) -> Vec<(Entity, &mut T)> {
        let storage = self
            .components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<Entity, T>::new()))
            .downcast_mut::<HashMap<Entity, T>>()
            .expect("TypeId mismatch or invalid component cast");

        storage.iter_mut().map(|(e, comp)| (*e, comp)).collect()
    }

    /// Immutable query for a single component type
    pub fn query<T: Component>(&self) -> Vec<(Entity, &T)> {
        if let Some(boxed) = self.components.get(&TypeId::of::<T>()) {
            if let Some(storage) = boxed.downcast_ref::<HashMap<Entity, T>>() {
                return storage.iter().map(|(e, c)| (*e, c)).collect();
            }
        }
        Vec::new()
    }

    /// Immutable query for two component types
    pub fn query2<A: Component, B: Component>(&self) -> Vec<(Entity, &A, &B)> {
        assert_ne!(TypeId::of::<A>(), TypeId::of::<B>());
        let a_map = if let Some(boxed) = self.components.get(&TypeId::of::<A>()) {
            boxed.downcast_ref::<HashMap<Entity, A>>()
        } else {
            None
        };
        let b_map = if let Some(boxed) = self.components.get(&TypeId::of::<B>()) {
            boxed.downcast_ref::<HashMap<Entity, B>>()
        } else {
            None
        };
        if let (Some(a_map), Some(b_map)) = (a_map, b_map) {
            let mut result = Vec::new();
            for (entity, a_val) in a_map.iter() {
                if let Some(b_val) = b_map.get(entity) {
                    result.push((*entity, a_val, b_val));
                }
            }
            return result;
        }
        Vec::new()
    }
    pub fn query2_mut<A: Component, B: Component>(&mut self) -> Vec<(Entity, &mut A, &mut B)> {
        assert_ne!(
            TypeId::of::<A>(),
            TypeId::of::<B>(),
            "query2_mut::<A, A>() is not allowed"
        );

        let a_ptr: *mut HashMap<Entity, A> = {
            let entry = self
                .components
                .entry(TypeId::of::<A>())
                .or_insert_with(|| Box::new(HashMap::<Entity, A>::new()));
            entry
                .downcast_mut::<HashMap<Entity, A>>()
                .expect("TypeId mismatch or invalid component cast for A")
        };

        let b_ptr: *mut HashMap<Entity, B> = {
            let entry = self
                .components
                .entry(TypeId::of::<B>())
                .or_insert_with(|| Box::new(HashMap::<Entity, B>::new()));
            entry
                .downcast_mut::<HashMap<Entity, B>>()
                .expect("TypeId mismatch or invalid component cast for B")
        };

        let (a_map, b_map): (&mut HashMap<Entity, A>, &mut HashMap<Entity, B>) =
            unsafe { (&mut *a_ptr, &mut *b_ptr) };

        let mut result = vec![];
        for (entity, a_val) in a_map.iter_mut() {
            if let Some(b_val_ptr) = b_map.get_mut(entity).map(|b| b as *mut B) {
                let a_val_ptr = a_val as *mut A;
                unsafe {
                    result.push((*entity, &mut *a_val_ptr, &mut *b_val_ptr));
                }
            }
        }

        result
    }

    pub fn query3_mut<A: Component, B: Component, C: Component>(
        &mut self,
    ) -> Vec<(Entity, &mut A, &mut B, &mut C)> {
        assert!(TypeId::of::<A>() != TypeId::of::<B>());
        assert!(TypeId::of::<A>() != TypeId::of::<C>());
        assert!(TypeId::of::<B>() != TypeId::of::<C>());

        let a_ptr: *mut HashMap<Entity, A> = {
            let entry = self
                .components
                .entry(TypeId::of::<A>())
                .or_insert_with(|| Box::new(HashMap::<Entity, A>::new()));
            entry
                .downcast_mut::<HashMap<Entity, A>>()
                .expect("TypeId mismatch for A")
        };

        let b_ptr: *mut HashMap<Entity, B> = {
            let entry = self
                .components
                .entry(TypeId::of::<B>())
                .or_insert_with(|| Box::new(HashMap::<Entity, B>::new()));
            entry
                .downcast_mut::<HashMap<Entity, B>>()
                .expect("TypeId mismatch for B")
        };

        let c_ptr: *mut HashMap<Entity, C> = {
            let entry = self
                .components
                .entry(TypeId::of::<C>())
                .or_insert_with(|| Box::new(HashMap::<Entity, C>::new()));
            entry
                .downcast_mut::<HashMap<Entity, C>>()
                .expect("TypeId mismatch for C")
        };

        let (a_map, b_map, c_map): (
            &mut HashMap<Entity, A>,
            &mut HashMap<Entity, B>,
            &mut HashMap<Entity, C>,
        ) = unsafe { (&mut *a_ptr, &mut *b_ptr, &mut *c_ptr) };

        let mut result = vec![];
        for (entity, a_val) in a_map.iter_mut() {
            if let (Some(b_val_ptr), Some(c_val_ptr)) = (
                b_map.get_mut(entity).map(|b| b as *mut B),
                c_map.get_mut(entity).map(|c| c as *mut C),
            ) {
                let a_val_ptr = a_val as *mut A;
                unsafe {
                    result.push((*entity, &mut *a_val_ptr, &mut *b_val_ptr, &mut *c_val_ptr));
                }
            }
        }

        result
    }

    /// Immutable query for three component types.
    pub fn query3<A: Component, B: Component, C: Component>(&self) -> Vec<(Entity, &A, &B, &C)> {
        assert!(TypeId::of::<A>() != TypeId::of::<B>());
        assert!(TypeId::of::<A>() != TypeId::of::<C>());
        assert!(TypeId::of::<B>() != TypeId::of::<C>());

        let a_map = self
            .components
            .get(&TypeId::of::<A>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, A>>());
        let b_map = self
            .components
            .get(&TypeId::of::<B>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, B>>());
        let c_map = self
            .components
            .get(&TypeId::of::<C>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, C>>());

        if let (Some(a_map), Some(b_map), Some(c_map)) = (a_map, b_map, c_map) {
            let mut result = Vec::new();
            for (entity, a_val) in a_map.iter() {
                if let (Some(b_val), Some(c_val)) = (b_map.get(entity), c_map.get(entity)) {
                    result.push((*entity, a_val, b_val, c_val));
                }
            }
            result
        } else {
            Vec::new()
        }
    }

    pub fn query4_mut<A: Component, B: Component, C: Component, D: Component>(
        &mut self,
    ) -> Vec<(Entity, &mut A, &mut B, &mut C, &mut D)> {
        assert!(TypeId::of::<A>() != TypeId::of::<B>());
        assert!(TypeId::of::<A>() != TypeId::of::<C>());
        assert!(TypeId::of::<A>() != TypeId::of::<D>());
        assert!(TypeId::of::<B>() != TypeId::of::<C>());
        assert!(TypeId::of::<B>() != TypeId::of::<D>());
        assert!(TypeId::of::<C>() != TypeId::of::<D>());

        let a_ptr: *mut HashMap<Entity, A> = {
            let entry = self
                .components
                .entry(TypeId::of::<A>())
                .or_insert_with(|| Box::new(HashMap::<Entity, A>::new()));
            entry
                .downcast_mut::<HashMap<Entity, A>>()
                .expect("TypeId mismatch for A")
        };

        let b_ptr: *mut HashMap<Entity, B> = {
            let entry = self
                .components
                .entry(TypeId::of::<B>())
                .or_insert_with(|| Box::new(HashMap::<Entity, B>::new()));
            entry
                .downcast_mut::<HashMap<Entity, B>>()
                .expect("TypeId mismatch for B")
        };

        let c_ptr: *mut HashMap<Entity, C> = {
            let entry = self
                .components
                .entry(TypeId::of::<C>())
                .or_insert_with(|| Box::new(HashMap::<Entity, C>::new()));
            entry
                .downcast_mut::<HashMap<Entity, C>>()
                .expect("TypeId mismatch for C")
        };

        let d_ptr: *mut HashMap<Entity, D> = {
            let entry = self
                .components
                .entry(TypeId::of::<D>())
                .or_insert_with(|| Box::new(HashMap::<Entity, D>::new()));
            entry
                .downcast_mut::<HashMap<Entity, D>>()
                .expect("TypeId mismatch for D")
        };

        let (a_map, b_map, c_map, d_map): (
            &mut HashMap<Entity, A>,
            &mut HashMap<Entity, B>,
            &mut HashMap<Entity, C>,
            &mut HashMap<Entity, D>,
        ) = unsafe { (&mut *a_ptr, &mut *b_ptr, &mut *c_ptr, &mut *d_ptr) };

        let mut result = vec![];
        for (entity, a_val) in a_map.iter_mut() {
            if let (Some(b_val_ptr), Some(c_val_ptr), Some(d_val_ptr)) = (
                b_map.get_mut(entity).map(|b| b as *mut B),
                c_map.get_mut(entity).map(|c| c as *mut C),
                d_map.get_mut(entity).map(|d| d as *mut D),
            ) {
                let a_val_ptr = a_val as *mut A;
                unsafe {
                    result.push((
                        *entity,
                        &mut *a_val_ptr,
                        &mut *b_val_ptr,
                        &mut *c_val_ptr,
                        &mut *d_val_ptr,
                    ));
                }
            }
        }

        result
    }

    /// Immutable query for four component types.
    pub fn query4<A: Component, B: Component, C: Component, D: Component>(
        &self,
    ) -> Vec<(Entity, &A, &B, &C, &D)> {
        assert!(TypeId::of::<A>() != TypeId::of::<B>());
        assert!(TypeId::of::<A>() != TypeId::of::<C>());
        assert!(TypeId::of::<A>() != TypeId::of::<D>());
        assert!(TypeId::of::<B>() != TypeId::of::<C>());
        assert!(TypeId::of::<B>() != TypeId::of::<D>());
        assert!(TypeId::of::<C>() != TypeId::of::<D>());

        let a_map = self
            .components
            .get(&TypeId::of::<A>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, A>>());
        let b_map = self
            .components
            .get(&TypeId::of::<B>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, B>>());
        let c_map = self
            .components
            .get(&TypeId::of::<C>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, C>>());
        let d_map = self
            .components
            .get(&TypeId::of::<D>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, D>>());

        if let (Some(a_map), Some(b_map), Some(c_map), Some(d_map)) = (a_map, b_map, c_map, d_map) {
            let mut result = Vec::new();
            for (entity, a_val) in a_map.iter() {
                if let (Some(b_val), Some(c_val), Some(d_val)) =
                    (b_map.get(entity), c_map.get(entity), d_map.get(entity))
                {
                    result.push((*entity, a_val, b_val, c_val, d_val));
                }
            }
            result
        } else {
            Vec::new()
        }
    }

    pub fn query5_mut<A: Component, B: Component, C: Component, D: Component, E: Component>(
        &mut self,
    ) -> Vec<(Entity, &mut A, &mut B, &mut C, &mut D, &mut E)> {
        assert!(TypeId::of::<A>() != TypeId::of::<B>());
        assert!(TypeId::of::<A>() != TypeId::of::<C>());
        assert!(TypeId::of::<A>() != TypeId::of::<D>());
        assert!(TypeId::of::<A>() != TypeId::of::<E>());
        assert!(TypeId::of::<B>() != TypeId::of::<C>());
        assert!(TypeId::of::<B>() != TypeId::of::<D>());
        assert!(TypeId::of::<B>() != TypeId::of::<E>());
        assert!(TypeId::of::<C>() != TypeId::of::<D>());
        assert!(TypeId::of::<C>() != TypeId::of::<E>());
        assert!(TypeId::of::<D>() != TypeId::of::<E>());

        let a_ptr: *mut HashMap<Entity, A> = {
            let entry = self
                .components
                .entry(TypeId::of::<A>())
                .or_insert_with(|| Box::new(HashMap::<Entity, A>::new()));
            entry
                .downcast_mut::<HashMap<Entity, A>>()
                .expect("TypeId mismatch for A")
        };

        let b_ptr: *mut HashMap<Entity, B> = {
            let entry = self
                .components
                .entry(TypeId::of::<B>())
                .or_insert_with(|| Box::new(HashMap::<Entity, B>::new()));
            entry
                .downcast_mut::<HashMap<Entity, B>>()
                .expect("TypeId mismatch for B")
        };

        let c_ptr: *mut HashMap<Entity, C> = {
            let entry = self
                .components
                .entry(TypeId::of::<C>())
                .or_insert_with(|| Box::new(HashMap::<Entity, C>::new()));
            entry
                .downcast_mut::<HashMap<Entity, C>>()
                .expect("TypeId mismatch for C")
        };

        let d_ptr: *mut HashMap<Entity, D> = {
            let entry = self
                .components
                .entry(TypeId::of::<D>())
                .or_insert_with(|| Box::new(HashMap::<Entity, D>::new()));
            entry
                .downcast_mut::<HashMap<Entity, D>>()
                .expect("TypeId mismatch for D")
        };

        let e_ptr: *mut HashMap<Entity, E> = {
            let entry = self
                .components
                .entry(TypeId::of::<E>())
                .or_insert_with(|| Box::new(HashMap::<Entity, E>::new()));
            entry
                .downcast_mut::<HashMap<Entity, E>>()
                .expect("TypeId mismatch for E")
        };

        let (a_map, b_map, c_map, d_map, e_map): (
            &mut HashMap<Entity, A>,
            &mut HashMap<Entity, B>,
            &mut HashMap<Entity, C>,
            &mut HashMap<Entity, D>,
            &mut HashMap<Entity, E>,
        ) = unsafe {
            (
                &mut *a_ptr,
                &mut *b_ptr,
                &mut *c_ptr,
                &mut *d_ptr,
                &mut *e_ptr,
            )
        };

        let mut result = vec![];
        for (entity, a_val) in a_map.iter_mut() {
            if let (Some(b_val_ptr), Some(c_val_ptr), Some(d_val_ptr), Some(e_val_ptr)) = (
                b_map.get_mut(entity).map(|b| b as *mut B),
                c_map.get_mut(entity).map(|c| c as *mut C),
                d_map.get_mut(entity).map(|d| d as *mut D),
                e_map.get_mut(entity).map(|e| e as *mut E),
            ) {
                let a_val_ptr = a_val as *mut A;
                unsafe {
                    result.push((
                        *entity,
                        &mut *a_val_ptr,
                        &mut *b_val_ptr,
                        &mut *c_val_ptr,
                        &mut *d_val_ptr,
                        &mut *e_val_ptr,
                    ));
                }
            }
        }

        result
    }

    /// Immutable query for five component types.
    pub fn query5<A: Component, B: Component, C: Component, D: Component, E: Component>(
        &self,
    ) -> Vec<(Entity, &A, &B, &C, &D, &E)> {
        assert!(TypeId::of::<A>() != TypeId::of::<B>());
        assert!(TypeId::of::<A>() != TypeId::of::<C>());
        assert!(TypeId::of::<A>() != TypeId::of::<D>());
        assert!(TypeId::of::<A>() != TypeId::of::<E>());
        assert!(TypeId::of::<B>() != TypeId::of::<C>());
        assert!(TypeId::of::<B>() != TypeId::of::<D>());
        assert!(TypeId::of::<B>() != TypeId::of::<E>());
        assert!(TypeId::of::<C>() != TypeId::of::<D>());
        assert!(TypeId::of::<C>() != TypeId::of::<E>());
        assert!(TypeId::of::<D>() != TypeId::of::<E>());

        let a_map = self
            .components
            .get(&TypeId::of::<A>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, A>>());
        let b_map = self
            .components
            .get(&TypeId::of::<B>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, B>>());
        let c_map = self
            .components
            .get(&TypeId::of::<C>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, C>>());
        let d_map = self
            .components
            .get(&TypeId::of::<D>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, D>>());
        let e_map = self
            .components
            .get(&TypeId::of::<E>())
            .and_then(|b| b.downcast_ref::<HashMap<Entity, E>>());

        if let (Some(a_map), Some(b_map), Some(c_map), Some(d_map), Some(e_map)) =
            (a_map, b_map, c_map, d_map, e_map)
        {
            let mut result = Vec::new();
            for (entity, a_val) in a_map.iter() {
                if let (Some(b_val), Some(c_val), Some(d_val), Some(e_val)) = (
                    b_map.get(entity),
                    c_map.get(entity),
                    d_map.get(entity),
                    e_map.get(entity),
                ) {
                    result.push((*entity, a_val, b_val, c_val, d_val, e_val));
                }
            }
            result
        } else {
            Vec::new()
        }
    }

    pub fn inspect_components_of(&mut self, entity: Entity, ui: &mut egui::Ui) {
        for (_type_id, storage) in self.components.iter_mut() {
            if let Some(inspect_map) =
                storage.downcast_mut::<HashMap<Entity, Box<dyn InspectableComponent>>>()
            {
                if let Some(component) = inspect_map.get_mut(&entity) {
                    component.inspect(ui);
                }
            }
        }
    }



    /// Remove an entity from the world. All components should be removed
    /// separately before calling this.
    pub fn delete_entity(&mut self, entity: Entity) {
        self.entities.retain(|&e| e != entity);
    }
}
