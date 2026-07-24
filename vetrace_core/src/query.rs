use std::any::TypeId;
use std::marker::PhantomData;

use crate::{Actor, Commands, Component, Engine, Entity, World};
use crate::actor::is_actor_managed_mutation;

/// Immutable tuple query specification. Implemented for one to four component
/// references. Every item starts with its `Actor` handle.
pub trait QuerySpec<'world> {
    type Item;
    fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item>;
}

impl<'world, A: Component> QuerySpec<'world> for &'world A {
    type Item = (Actor, &'world A);
    fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item> {
        Some((Actor::from_entity(entity), world.get::<A>(entity)?))
    }
}

impl<'world, A: Component, B: Component> QuerySpec<'world> for (&'world A, &'world B) {
    type Item = (Actor, &'world A, &'world B);
    fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item> {
        Some((Actor::from_entity(entity), world.get::<A>(entity)?, world.get::<B>(entity)?))
    }
}

impl<'world, A: Component, B: Component, C: Component> QuerySpec<'world>
    for (&'world A, &'world B, &'world C)
{
    type Item = (Actor, &'world A, &'world B, &'world C);
    fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item> {
        Some((
            Actor::from_entity(entity),
            world.get::<A>(entity)?,
            world.get::<B>(entity)?,
            world.get::<C>(entity)?,
        ))
    }
}

impl<'world, A: Component, B: Component, C: Component, D: Component> QuerySpec<'world>
    for (&'world A, &'world B, &'world C, &'world D)
{
    type Item = (Actor, &'world A, &'world B, &'world C, &'world D);
    fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item> {
        Some((
            Actor::from_entity(entity),
            world.get::<A>(entity)?,
            world.get::<B>(entity)?,
            world.get::<C>(entity)?,
            world.get::<D>(entity)?,
        ))
    }
}

pub struct Query<'world, Q> {
    engine: &'world Engine,
    with: Vec<TypeId>,
    without: Vec<TypeId>,
    marker: PhantomData<Q>,
}

impl<'world, Q> Query<'world, Q>
where
    Q: QuerySpec<'world>,
{
    pub(crate) fn new(engine: &'world Engine) -> Self {
        Self { engine, with: Vec::new(), without: Vec::new(), marker: PhantomData }
    }

    pub fn with<T: Component>(mut self) -> Self {
        self.with.push(TypeId::of::<T>());
        self
    }

    pub fn without<T: Component>(mut self) -> Self {
        self.without.push(TypeId::of::<T>());
        self
    }

    pub fn collect(self) -> Vec<Q::Item> {
        let world = self.engine.world_ref();
        world
            .entities()
            .filter(|entity| self.with.iter().all(|id| world.has_type(*entity, *id)))
            .filter(|entity| self.without.iter().all(|id| !world.has_type(*entity, *id)))
            .filter_map(|entity| Q::fetch(world, entity))
            .collect()
    }
}

impl<'world, Q> IntoIterator for Query<'world, Q>
where
    Q: QuerySpec<'world>,
{
    type Item = Q::Item;
    type IntoIter = std::vec::IntoIter<Q::Item>;

    fn into_iter(self) -> Self::IntoIter { self.collect().into_iter() }
}

/// Mutable single-component query. Mutable queries use a callback so component
/// references cannot escape while structural commands are deferred.
pub struct MutQuery<'world, A: Component> {
    engine: &'world mut Engine,
    with: Vec<TypeId>,
    without: Vec<TypeId>,
    marker: PhantomData<A>,
}

impl<'world, A: Component> MutQuery<'world, A> {
    pub(crate) fn new(engine: &'world mut Engine) -> Self {
        assert!(!is_actor_managed_mutation(TypeId::of::<A>()), "managed Actor components require their dedicated API");
        Self { engine, with: Vec::new(), without: Vec::new(), marker: PhantomData }
    }

    pub fn with<T: Component>(mut self) -> Self {
        self.with.push(TypeId::of::<T>());
        self
    }

    pub fn without<T: Component>(mut self) -> Self {
        self.without.push(TypeId::of::<T>());
        self
    }

    pub fn for_each(mut self, mut visit: impl FnMut(Actor, &mut A)) {
        let entities = matching_entities(self.engine, &self.with, &self.without);
        for entity in entities {
            let Some(component) = self.engine.world_mut().get_mut::<A>(entity) else { continue; };
            visit(Actor::from_entity(entity), component);
        }
    }

    /// Visit matching components while queuing structural changes safely.
    /// Commands are appended to the engine after the active component borrow
    /// ends and are flushed at the normal stage boundary.
    pub fn for_each_with_commands(mut self, mut visit: impl FnMut(Actor, &mut A, &mut Commands<'_>)) {
        let entities = matching_entities(self.engine, &self.with, &self.without);
        let mut queued = Vec::new();
        for entity in entities {
            let Some(component) = self.engine.world_mut().get_mut::<A>(entity) else { continue; };
            let mut commands = Commands::new(&mut queued);
            visit(Actor::from_entity(entity), component, &mut commands);
        }
        self.engine.pending_commands.extend(queued);
    }
}

/// Mutable query with one mutable and one immutable component.
///
/// `A` and `B` must be different component types. Structural changes should be
/// queued through `Commands` and are applied after the active stage.
pub struct MutQueryWith<'world, A: Component, B: Component> {
    engine: &'world mut Engine,
    with: Vec<TypeId>,
    without: Vec<TypeId>,
    marker: PhantomData<(A, B)>,
}

impl<'world, A: Component, B: Component> MutQueryWith<'world, A, B> {
    pub(crate) fn new(engine: &'world mut Engine) -> Self {
        assert_ne!(TypeId::of::<A>(), TypeId::of::<B>(), "mutable query component types must be distinct");
        assert!(!is_actor_managed_mutation(TypeId::of::<A>()), "managed Actor components require their dedicated API");
        Self { engine, with: Vec::new(), without: Vec::new(), marker: PhantomData }
    }

    pub fn with<T: Component>(mut self) -> Self {
        self.with.push(TypeId::of::<T>());
        self
    }

    pub fn without<T: Component>(mut self) -> Self {
        self.without.push(TypeId::of::<T>());
        self
    }

    pub fn for_each(mut self, mut visit: impl FnMut(Actor, &mut A, &B)) {
        let entities = matching_entities(self.engine, &self.with, &self.without);
        for entity in entities {
            // The type IDs are guaranteed distinct above. The callback cannot
            // access Engine, so no structural mutation can invalidate either
            // pointer during the call.
            let Some(read_ptr) = self.engine.world_ref().get::<B>(entity).map(|value| value as *const B) else {
                continue;
            };
            let Some(write_ptr) = self.engine.world_mut().get_mut::<A>(entity).map(|value| value as *mut A) else {
                continue;
            };
            unsafe { visit(Actor::from_entity(entity), &mut *write_ptr, &*read_ptr); }
        }
    }

    /// Mutable/immutable tuple query with deferred structural commands.
    pub fn for_each_with_commands(
        mut self,
        mut visit: impl FnMut(Actor, &mut A, &B, &mut Commands<'_>),
    ) {
        let entities = matching_entities(self.engine, &self.with, &self.without);
        let mut queued = Vec::new();
        for entity in entities {
            let Some(read_ptr) = self.engine.world_ref().get::<B>(entity).map(|value| value as *const B) else {
                continue;
            };
            let Some(write_ptr) = self.engine.world_mut().get_mut::<A>(entity).map(|value| value as *mut A) else {
                continue;
            };
            let mut commands = Commands::new(&mut queued);
            unsafe { visit(Actor::from_entity(entity), &mut *write_ptr, &*read_ptr, &mut commands); }
        }
        self.engine.pending_commands.extend(queued);
    }
}

fn matching_entities(engine: &Engine, with: &[TypeId], without: &[TypeId]) -> Vec<Entity> {
    let world = engine.world_ref();
    world
        .entities()
        .filter(|entity| with.iter().all(|type_id| world.has_type(*entity, *type_id)))
        .filter(|entity| without.iter().all(|type_id| !world.has_type(*entity, *type_id)))
        .collect()
}
