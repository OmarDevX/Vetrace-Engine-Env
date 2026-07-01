//! Component synchronization traits and utilities.

/// Trait for components that can be synchronized over the network.
pub trait NetSyncComponent {
    /// Serialize the component into bytes.
    fn serialize(&self) -> Vec<u8>;
    /// Deserialize from bytes into the component.
    fn deserialize(&mut self, data: &[u8]);
    /// Returns true if the component has changed since last sync.
    fn has_changed(&self) -> bool;
    /// Static name of the component type for identification.
    fn component_name() -> &'static str
    where
        Self: Sized;
}

use std::collections::HashMap;

use super::packets::EntitySnapshot;
use crate::ecs::{Component, Entity, World};

pub struct NetSyncHooks {
    pub collect: Box<dyn Fn(&mut World) -> Vec<(Entity, Vec<u8>)>>,
    pub apply: Box<dyn Fn(&mut World, Entity, &[u8])>,
}

#[derive(Default)]
pub struct NetSyncRegistry {
    pub components: HashMap<&'static str, NetSyncHooks>,
}

fn collect_component<T: Component + NetSyncComponent>(world: &mut World) -> Vec<(Entity, Vec<u8>)> {
    world
        .query::<T>()
        .into_iter()
        .filter_map(|(e, comp)| {
            if comp.has_changed() {
                Some((e, comp.serialize()))
            } else {
                None
            }
        })
        .collect()
}

fn apply_component<T: Component + NetSyncComponent>(
    world: &mut World,
    entity: Entity,
    data: &[u8],
) {
    if let Some(comp) = world.get_mut::<T>(entity) {
        comp.deserialize(data);
    }
}

pub fn register_sync_component<T>(registry: &mut NetSyncRegistry)
where
    T: Component + NetSyncComponent + 'static,
{
    register_sync_component_with_filter::<T, _>(registry, |_, _e, _c| true);
}

pub fn register_sync_component_with_filter<T, F>(registry: &mut NetSyncRegistry, filter: F)
where
    T: Component + NetSyncComponent + 'static,
    F: Fn(&World, Entity, &T) -> bool + 'static,
{
    registry.components.insert(
        T::component_name(),
        NetSyncHooks {
            collect: Box::new(move |world: &mut World| {
                world
                    .query::<T>()
                    .into_iter()
                    .filter_map(|(e, comp)| {
                        if comp.has_changed() && filter(&*world, e, comp) {
                            Some((e, comp.serialize()))
                        } else {
                            None
                        }
                    })
                    .collect()
            }),
            apply: Box::new(|world: &mut World, entity: Entity, data: &[u8]| {
                if let Some(comp) = world.get_mut::<T>(entity) {
                    comp.deserialize(data);
                }
            }),
        },
    );
}

/// Collect snapshots for all registered components in the given world.
pub fn collect_snapshots(world: &mut World, registry: &NetSyncRegistry) -> Vec<EntitySnapshot> {
    let mut map: HashMap<Entity, Vec<(String, Vec<u8>)>> = HashMap::new();
    for (name, hooks) in &registry.components {
        for (entity, data) in (hooks.collect)(world) {
            map.entry(entity)
                .or_default()
                .push(((*name).to_string(), data));
        }
    }
    map.into_iter()
        .map(|(e, components)| EntitySnapshot {
            entity: e.0,
            components,
        })
        .collect()
}

/// Apply snapshots to the given world using the registry for component hooks.
pub fn apply_snapshots(
    world: &mut World,
    registry: &NetSyncRegistry,
    snapshots: &[EntitySnapshot],
) {
    for snap in snapshots {
        let entity = Entity(snap.entity);
        for (name, data) in &snap.components {
            if let Some(hooks) = registry.components.get(name.as_str()) {
                (hooks.apply)(world, entity, data);
            }
        }
    }
}
