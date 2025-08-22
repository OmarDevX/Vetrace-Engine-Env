use super::{Actor, Engine};
use crate::ecs::{Component, Entity};
use std::marker::PhantomData;

/// High-level wrapper similar to [`World`] that yields [`Actor`]s
/// instead of raw entity ids.
pub struct Stage<'a> {
    engine: *mut Engine,
    _marker: PhantomData<&'a mut Engine>,
}

impl<'a> Stage<'a> {
    pub(crate) fn new(engine: &'a mut Engine) -> Self {
        Self {
            engine: engine as *mut Engine,
            _marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn engine_mut(&self) -> &'a mut Engine {
        unsafe { &mut *self.engine }
    }

    /// Spawn an empty actor with a name.
    pub fn spawn_actor(&mut self, name: &str) -> Actor<'a> {
        unsafe {
            let engine = self.engine_mut();
            let entity = engine.spawn_empty(name);
            Actor::new(engine, entity)
        }
    }

    /// Wrap an existing entity as an [`Actor`].
    pub fn actor_from_entity(&mut self, entity: Entity) -> Actor<'a> {
        unsafe { Actor::new(self.engine_mut(), entity) }
    }

    /// Try to fetch an [`Actor`] if the entity exists.
    pub fn get_actor(&mut self, entity: Entity) -> Option<Actor<'a>> {
        unsafe {
            let ptr = self.engine;
            if (&*ptr).world.entities().contains(&entity) {
                Some(Actor::new(&mut *ptr, entity))
            } else {
                None
            }
        }
    }

    /// Get an [`Actor`] by object index if it exists.
    pub fn actor_from_object(&mut self, index: usize) -> Option<Actor<'a>> {
        unsafe { Actor::from_object_index(self.engine_mut(), index) }
    }

    /// Load a scene file using the underlying engine utilities.
    pub fn load_scene(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { self.engine_mut().load_scene_from_file(path) }
    }

    /// Remove an actor and all its components.
    pub fn despawn_actor(&mut self, entity: Entity) {
        unsafe {
            self.engine_mut().delete_entity(entity);
        }
    }

    pub fn query_mut<T: Component + 'static>(&mut self) -> Vec<(Actor<'a>, &mut T)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query_mut::<T>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, c)| (Actor::new(&mut *ptr, e), &mut *(c as *mut T)))
                .collect()
        }
    }

    pub fn query<T: Component + 'static>(&mut self) -> Vec<(Actor<'a>, &T)> {
        unsafe {
            let ptr = self.engine;
            let results = (&*ptr).world.query::<T>();
            results
                .into_iter()
                .map(|(e, c)| (Actor::new(&mut *ptr, e), c))
                .collect()
        }
    }

    pub fn query2_mut<A: Component + 'static, B: Component + 'static>(
        &mut self,
    ) -> Vec<(Actor<'a>, &mut A, &mut B)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query2_mut::<A, B>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &mut *(a as *mut A),
                        &mut *(b as *mut B),
                    )
                })
                .collect()
        }
    }

    pub fn query2<A: Component + 'static, B: Component + 'static>(
        &mut self,
    ) -> Vec<(Actor<'a>, &A, &B)> {
        unsafe {
            let ptr = self.engine;
            let results = (&*ptr).world.query2::<A, B>();
            results
                .into_iter()
                .map(|(e, a, b)| (Actor::new(&mut *ptr, e), a, b))
                .collect()
        }
    }

    pub fn query3_mut<A: Component + 'static, B: Component + 'static, C: Component + 'static>(
        &mut self,
    ) -> Vec<(Actor<'a>, &mut A, &mut B, &mut C)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query3_mut::<A, B, C>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b, c)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &mut *(a as *mut A),
                        &mut *(b as *mut B),
                        &mut *(c as *mut C),
                    )
                })
                .collect()
        }
    }

    pub fn query3<A: Component + 'static, B: Component + 'static, C: Component + 'static>(
        &mut self,
    ) -> Vec<(Actor<'a>, &A, &B, &C)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query3::<A, B, C>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b, c)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &*(a as *const A),
                        &*(b as *const B),
                        &*(c as *const C),
                    )
                })
                .collect()
        }
    }

    pub fn query4_mut<
        A: Component + 'static,
        B: Component + 'static,
        C: Component + 'static,
        D: Component + 'static,
    >(
        &mut self,
    ) -> Vec<(Actor<'a>, &mut A, &mut B, &mut C, &mut D)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query4_mut::<A, B, C, D>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b, c, d)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &mut *(a as *mut A),
                        &mut *(b as *mut B),
                        &mut *(c as *mut C),
                        &mut *(d as *mut D),
                    )
                })
                .collect()
        }
    }

    pub fn query4<
        A: Component + 'static,
        B: Component + 'static,
        C: Component + 'static,
        D: Component + 'static,
    >(
        &mut self,
    ) -> Vec<(Actor<'a>, &A, &B, &C, &D)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query4::<A, B, C, D>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b, c, d)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &*(a as *const A),
                        &*(b as *const B),
                        &*(c as *const C),
                        &*(d as *const D),
                    )
                })
                .collect()
        }
    }

    pub fn query5_mut<
        A: Component + 'static,
        B: Component + 'static,
        C: Component + 'static,
        D: Component + 'static,
        E: Component + 'static,
    >(
        &mut self,
    ) -> Vec<(Actor<'a>, &mut A, &mut B, &mut C, &mut D, &mut E)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query5_mut::<A, B, C, D, E>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b, c, d, e2)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &mut *(a as *mut A),
                        &mut *(b as *mut B),
                        &mut *(c as *mut C),
                        &mut *(d as *mut D),
                        &mut *(e2 as *mut E),
                    )
                })
                .collect()
        }
    }

    pub fn query5<
        A: Component + 'static,
        B: Component + 'static,
        C: Component + 'static,
        D: Component + 'static,
        E: Component + 'static,
    >(
        &mut self,
    ) -> Vec<(Actor<'a>, &A, &B, &C, &D, &E)> {
        unsafe {
            let engine = self.engine_mut();
            let results = engine.world.query5::<A, B, C, D, E>();
            let ptr = self.engine;
            results
                .into_iter()
                .map(|(e, a, b, c, d, e2)| {
                    (
                        Actor::new(&mut *ptr, e),
                        &*(a as *const A),
                        &*(b as *const B),
                        &*(c as *const C),
                        &*(d as *const D),
                        &*(e2 as *const E),
                    )
                })
                .collect()
        }
    }

    /// Find an actor by name if it exists.
    pub fn find_actor_by_name(&mut self, name: &str) -> Option<Actor<'a>> {
        use crate::components::components::Metadata;
        unsafe {
            let engine = self.engine_mut();
            for &e in engine.world.entities() {
                if let Some(meta) = engine.world.get::<Metadata>(e) {
                    if meta.name == name {
                        let ptr = self.engine;
                        return Some(Actor::new(&mut *ptr, e));
                    }
                }
            }
            None
        }
    }

    /// Find the first actor tagged with `tag`.
    pub fn find_actor_with_tag(&mut self, tag: &str) -> Option<Actor<'a>> {
        use crate::components::components::Metadata;
        unsafe {
            let engine = self.engine_mut();
            for &e in engine.world.entities() {
                if let Some(meta) = engine.world.get::<Metadata>(e) {
                    if meta.tags.iter().any(|t| t == tag) {
                        let ptr = self.engine;
                        return Some(Actor::new(&mut *ptr, e));
                    }
                }
            }
            None
        }
    }

    /// Retrieve all actors that contain the specified tag.
    pub fn find_all_with_tag(&mut self, tag: &str) -> Vec<Actor<'a>> {
        use crate::components::components::Metadata;
        unsafe {
            let engine = self.engine_mut();
            let mut result = Vec::new();
            for &e in engine.world.entities() {
                if let Some(meta) = engine.world.get::<Metadata>(e) {
                    if meta.tags.iter().any(|t| t == tag) {
                        let ptr = self.engine;
                        result.push(Actor::new(&mut *ptr, e));
                    }
                }
            }
            result
        }
    }
}
