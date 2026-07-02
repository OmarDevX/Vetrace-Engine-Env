// Actor wrapper providing a Unity-like API around ECS entities.
use super::Engine;
use crate::components::components::{Children, Metadata, Parent};
use crate::ecs::{Component, Entity};

/// Trait allowing a bundle of components to be inserted on an [`Actor`].
pub trait ComponentBundle {
    fn insert(self, engine: &mut Engine, entity: Entity);
}

impl<T: Component + 'static> ComponentBundle for T {
    fn insert(self, engine: &mut Engine, entity: Entity) {
        engine.world.insert(entity, self);
    }
}

macro_rules! impl_bundle {
    ( $( $name:ident ),+ ) => {
        impl<$( $name: Component + 'static ),+> ComponentBundle for ( $( $name , )+ ) {
            #[allow(non_snake_case)]
            fn insert(self, engine: &mut Engine, entity: Entity) {
                let ( $( $name , )+ ) = self;
                $( engine.world.insert(entity, $name); )+
            }
        }
    };
}

impl_bundle!(A, B);
impl_bundle!(A, B, C);
impl_bundle!(A, B, C, D);
impl_bundle!(A, B, C, D, E);

/// Helper struct similar to Unity's `GameObject`.
///
/// An `Actor` owns a reference to the [`Engine`] and a specific [`Entity`].
/// It exposes convenience methods for manipulating components on that entity.
pub struct Actor<'a> {
    pub(crate) engine: &'a mut Engine,
    entity: Entity,
}

impl<'a> Actor<'a> {
    /// Create a new `Actor` from an existing entity.
    pub fn new(engine: &'a mut Engine, entity: Entity) -> Self {
        Self { engine, entity }
    }

    /// Return the wrapped entity identifier.
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Try to resolve an object index to an `Actor`.
    pub fn from_object_index(engine: &'a mut Engine, index: usize) -> Option<Self> {
        engine
            .core
            .find_entity_by_object_id(index as u32)
            .map(|e| Self { engine, entity: e })
    }

    /// Insert a default component `T` on this actor if it doesn't exist.
    pub fn add_component<T: Component + Default + 'static>(&mut self) {
        self.engine.add_component_entity::<T>(self.entity);
    }

    /// Insert a bundle of components at once.
    pub fn with_bundle<B: ComponentBundle>(&mut self, bundle: B) -> &mut Self {
        bundle.insert(self.engine, self.entity);
        self
    }

    /// Remove component `T` from this actor.
    pub fn remove_component<T: Component + 'static>(&mut self) {
        self.engine.remove_component_entity::<T>(self.entity);
    }

    /// Borrow a mutable reference to component `T` if present.
    pub fn get_component_mut<T: Component + 'static>(&mut self) -> Option<&mut T> {
        self.engine.get_component_mut_entity::<T>(self.entity)
    }

    /// Borrow an immutable reference to component `T` if present.
    pub fn get_component<T: Component + 'static>(&self) -> Option<&T> {
        self.engine.world.get::<T>(self.entity)
    }

    /// Check whether this actor contains component `T`.
    pub fn has_component<T: Component + 'static>(&self) -> bool {
        self.engine.world.has::<T>(self.entity)
    }

    /// Retrieve the list of component names currently attached to this actor.
    pub fn list_components(&self) -> Vec<String> {
        self.engine.list_components_entity(self.entity)
    }

    /// Get this actor's name if a [`Metadata`] component is present.
    pub fn name(&self) -> Option<&str> {
        self.get_component::<Metadata>().map(|m| m.name.as_str())
    }

    /// Set or replace the [`Parent`] component pointing to `parent`.
    pub fn set_parent(&mut self, parent: &Actor) {
        if let Some(p) = self.engine.world.get_mut::<Parent>(self.entity) {
            p.entity = parent.entity();
        } else {
            self.engine.world.insert(
                self.entity,
                Parent {
                    entity: parent.entity(),
                },
            );
        }
    }

    /// Retrieve the parent entity if one is set.
    pub fn parent(&self) -> Option<Entity> {
        self.get_component::<Parent>().map(|p| p.entity)
    }

    /// Return this actor's children as new `Actor` wrappers.
    pub fn children(&'a mut self) -> Option<Vec<Actor<'a>>> {
        if let Some(c) = self.engine.world.get::<Children>(self.entity) {
            let ids: Vec<_> = c.entities.iter().copied().collect();
            let ptr = self.engine as *mut Engine;
            Some(
                ids.into_iter()
                    .map(|e| unsafe { Actor::new(&mut *ptr, e) })
                    .collect(),
            )
        } else {
            None
        }
    }

    /// Send a named event to `target`.
    pub fn send_event(&mut self, name: &str, target: Entity) {
        self.engine
            .entity_events
            .push((self.entity, target, name.to_string()));
        self.engine
            .entity_event
            .emit((self.entity, target, name.to_string()));
    }
}
