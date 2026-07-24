use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use vetrace_core::{Entity, World};

use crate::protocol::TransferMode;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicatedComponentConfig {
    pub component_name: String,
    pub enabled: bool,
    pub transfer_mode: TransferMode,
    pub interpolation_seconds: Option<f32>,
}

impl ReplicatedComponentConfig {
    pub fn new(component_name: impl Into<String>) -> Self {
        Self {
            component_name: component_name.into(),
            enabled: true,
            transfer_mode: TransferMode::UnreliableOrdered,
            interpolation_seconds: None,
        }
    }

    pub fn reliable(mut self) -> Self {
        self.transfer_mode = TransferMode::Reliable;
        self
    }

    pub fn unreliable(mut self) -> Self {
        self.transfer_mode = TransferMode::Unreliable;
        self
    }

    pub fn unreliable_ordered(mut self) -> Self {
        self.transfer_mode = TransferMode::UnreliableOrdered;
        self
    }

    pub fn interpolated(mut self, seconds: f32) -> Self {
        self.interpolation_seconds = Some(seconds.max(0.001));
        self
    }

    pub fn immediate(mut self) -> Self {
        self.interpolation_seconds = None;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Default for ReplicatedComponentConfig {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReplicatedComponentList {
    pub components: Vec<ReplicatedComponentConfig>,
}

impl ReplicatedComponentList {
    pub fn new() -> Self { Self::default() }

    pub fn upsert(&mut self, config: ReplicatedComponentConfig) {
        if let Some(existing) = self.components.iter_mut().find(|item| item.component_name == config.component_name) {
            *existing = config;
        } else {
            self.components.push(config);
        }
    }

    pub fn remove(&mut self, component_name: &str) -> Option<ReplicatedComponentConfig> {
        let index = self.components.iter().position(|item| item.component_name == component_name)?;
        Some(self.components.remove(index))
    }

    pub fn get(&self, component_name: &str) -> Option<&ReplicatedComponentConfig> {
        self.components.iter().find(|item| item.component_name == component_name)
    }

    pub fn enabled_components(&self) -> impl Iterator<Item = &ReplicatedComponentConfig> {
        self.components.iter().filter(|item| item.enabled)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplicatedComponentSnapshot<T> {
    pub net_id: u64,
    pub tick: u64,
    pub component_name: String,
    pub data: T,
}

impl<T> ReplicatedComponentSnapshot<T> {
    pub fn new(net_id: u64, tick: u64, component_name: impl Into<String>, data: T) -> Self {
        Self { net_id, tick, component_name: component_name.into(), data }
    }

    pub fn net_id(&self) -> u64 { self.net_id }
    pub fn tick(&self) -> u64 { self.tick }
    pub fn component_name(&self) -> &str { &self.component_name }
}

pub struct ComponentSnapshotRef<'a, T> {
    pub net_id: u64,
    pub tick: u64,
    pub component_name: &'a str,
    pub data: &'a T,
}

/// Game/engine-side adapter for one replicated component type.
///
/// `vetrace_net` owns only this trait. Concrete adapters live outside the net
/// crate, for example a transform adapter in an engine integration crate or a
/// gameplay-state adapter in a game.
pub trait ReplicatedComponentAdapter {
    type Snapshot: Serialize + DeserializeOwned + Clone + 'static;

    fn component_name() -> &'static str;

    fn capture(world: &World, entity: Entity, net_id: u64, tick: u64) -> Option<ReplicatedComponentSnapshot<Self::Snapshot>>;

    fn apply(world: &mut World, entity: Entity, snapshot: &Self::Snapshot);

    fn interpolate(from: &Self::Snapshot, to: &Self::Snapshot, alpha: f32) -> Self::Snapshot {
        let _ = (from, alpha);
        to.clone()
    }
}
