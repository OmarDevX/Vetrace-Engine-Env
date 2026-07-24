use std::any::{Any, TypeId};
use std::collections::{BTreeSet, HashMap};

/// Generational runtime entity handle.
///
/// The low 32 bits store the slot index and the high 32 bits store its
/// generation. Keeping the packed `u64` representation preserves compatibility
/// with low-level backends while preventing stale handles from becoming valid
/// again after an entity slot is reused.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Entity(pub u64);

impl Entity {
    pub const INVALID: Self = Self(0);

    pub const fn from_parts(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << 32) | index as u64)
    }

    pub const fn index(self) -> u32 { self.0 as u32 }
    pub const fn generation(self) -> u32 { (self.0 >> 32) as u32 }
    pub const fn raw(self) -> u64 { self.0 }
    pub const fn is_valid(self) -> bool { self.generation() != 0 }
}

/// Marker trait for ECS components.
pub trait Component: Any + 'static {}
impl<T: Any + 'static> Component for T {}

#[derive(Clone, Copy, Debug)]
struct EntitySlot {
    generation: u32,
    alive: bool,
}

#[derive(Default)]
pub struct World {
    slots: Vec<EntitySlot>,
    free: Vec<u32>,
    alive: BTreeSet<Entity>,
    storages: HashMap<TypeId, HashMap<Entity, Box<dyn Any>>>,
    changed: HashMap<TypeId, BTreeSet<Entity>>,
}

impl World {
    pub fn new() -> Self { Self::default() }

    pub fn spawn(&mut self) -> Entity {
        let index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(EntitySlot { generation: 1, alive: false });
            index
        };

        let slot = &mut self.slots[index as usize];
        slot.alive = true;
        let entity = Entity::from_parts(index, slot.generation.max(1));
        self.alive.insert(entity);
        entity
    }

    pub fn clear(&mut self) {
        self.alive.clear();
        self.storages.clear();
        self.changed.clear();
        self.free.clear();
        for (index, slot) in self.slots.iter_mut().enumerate() {
            slot.alive = false;
            slot.generation = next_generation(slot.generation);
            self.free.push(index as u32);
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) { return false; }
        self.alive.remove(&entity);
        for storage in self.storages.values_mut() {
            storage.remove(&entity);
        }
        for changed in self.changed.values_mut() {
            changed.remove(&entity);
        }
        let slot = &mut self.slots[entity.index() as usize];
        slot.alive = false;
        slot.generation = next_generation(slot.generation);
        self.free.push(entity.index());
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        let Some(slot) = self.slots.get(entity.index() as usize) else { return false; };
        entity.is_valid() && slot.alive && slot.generation == entity.generation()
    }

    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ { self.alive.iter().copied() }

    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        assert!(self.is_alive(entity), "cannot insert component on dead entity {:?}", entity);
        let type_id = TypeId::of::<T>();
        self.storages.entry(type_id).or_default().insert(entity, Box::new(component));
        self.changed.entry(type_id).or_default().insert(entity);
    }

    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.is_alive(entity) { return None; }
        self.storages.get(&TypeId::of::<T>())?.get(&entity)?.downcast_ref::<T>()
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.is_alive(entity) { return None; }
        let type_id = TypeId::of::<T>();
        if !self.storages.get(&type_id)?.contains_key(&entity) { return None; }
        self.changed.entry(type_id).or_default().insert(entity);
        self.storages.get_mut(&type_id)?.get_mut(&entity)?.downcast_mut::<T>()
    }

    pub fn has<T: Component>(&self, entity: Entity) -> bool { self.get::<T>(entity).is_some() }

    pub fn has_type(&self, entity: Entity, type_id: TypeId) -> bool {
        self.is_alive(entity) && self.storages.get(&type_id).map(|s| s.contains_key(&entity)).unwrap_or(false)
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) { return None; }
        let type_id = TypeId::of::<T>();
        let removed = self.storages
            .get_mut(&type_id)?
            .remove(&entity)
            .map(|old| *old.downcast::<T>().expect("component type mismatch"));
        if removed.is_some() { self.changed.entry(type_id).or_default().insert(entity); }
        removed
    }

    pub fn query<T: Component>(&self) -> Vec<(Entity, &T)> {
        let Some(storage) = self.storages.get(&TypeId::of::<T>()) else { return Vec::new(); };
        storage
            .iter()
            .filter(|(entity, _)| self.is_alive(**entity))
            .filter_map(|(entity, value)| value.downcast_ref::<T>().map(|component| (*entity, component)))
            .collect()
    }

    pub fn query_mut<T: Component>(&mut self) -> Vec<(Entity, &mut T)> {
        let type_id = TypeId::of::<T>();
        let entities = self
            .storages
            .get(&type_id)
            .map(|storage| storage.keys().copied().filter(|entity| self.alive.contains(entity)).collect::<Vec<_>>())
            .unwrap_or_default();
        self.changed.entry(type_id).or_default().extend(entities);
        let alive = &self.alive;
        let Some(storage) = self.storages.get_mut(&type_id) else { return Vec::new(); };
        storage
            .iter_mut()
            .filter(|(entity, _)| alive.contains(entity))
            .filter_map(|(entity, value)| value.downcast_mut::<T>().map(|component| (*entity, component)))
            .collect()
    }

    pub fn changed<T: Component>(&self) -> impl Iterator<Item = Entity> + '_ {
        self.changed
            .get(&TypeId::of::<T>())
            .into_iter()
            .flat_map(|entities| entities.iter().copied())
    }

    pub fn take_changed<T: Component>(&mut self) -> Vec<Entity> {
        self.changed.remove(&TypeId::of::<T>()).map(|entities| entities.into_iter().collect()).unwrap_or_default()
    }
}


fn next_generation(generation: u32) -> u32 {
    let next = generation.wrapping_add(1);
    if next == 0 { 1 } else { next }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_handles_do_not_revive_after_reuse() {
        let mut world = World::new();
        let first = world.spawn();
        assert!(world.despawn(first));
        let second = world.spawn();
        assert_eq!(first.index(), second.index());
        assert_ne!(first.generation(), second.generation());
        assert!(!world.is_alive(first));
        assert!(world.is_alive(second));
    }

    #[test]
    fn clear_invalidates_all_existing_handles() {
        let mut world = World::new();
        let actor = world.spawn();
        world.clear();
        assert!(!world.is_alive(actor));
        let replacement = world.spawn();
        assert_ne!(actor, replacement);
    }
}
