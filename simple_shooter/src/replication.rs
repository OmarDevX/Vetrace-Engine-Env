//! Simple Shooter's concrete replicated-component adapters.
//!
//! `vetrace_net` only knows about generic replicated component snapshots.
//! This file is where the game chooses to replicate the core `Transform`
//! component and defines how that snapshot is captured, applied, and
//! interpolated.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use vetrace_core::{Actor, Engine, Entity, Transform, World};
use vetrace_net::{
    GenericComponentInterpolator, ReplicatedComponentAdapter, ReplicatedComponentConfig,
    ReplicatedComponentSnapshot,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformSnapshot {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl TransformSnapshot {
    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            translation: vec3_to_array(transform.translation),
            rotation: quat_to_array(transform.rotation),
            scale: vec3_to_array(transform.scale),
        }
    }

    pub fn translation_vec3(&self) -> Vec3 {
        Vec3::new(self.translation[0], self.translation[1], self.translation[2])
    }

    pub fn rotation_quat(&self) -> Quat {
        quat_from_array(self.rotation)
    }

    pub fn scale_vec3(&self) -> Vec3 {
        Vec3::new(self.scale[0], self.scale[1], self.scale[2])
    }

    pub fn interpolate(from: &Self, to: &Self, alpha: f32) -> Self {
        let t = alpha.clamp(0.0, 1.0);
        let from_rot = from.rotation_quat();
        let to_rot = to.rotation_quat();
        Self {
            translation: vec3_to_array(from.translation_vec3().lerp(to.translation_vec3(), t)),
            rotation: quat_to_array(from_rot.slerp(to_rot, t)),
            scale: vec3_to_array(from.scale_vec3().lerp(to.scale_vec3(), t)),
        }
    }
}

impl Default for TransformSnapshot {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: quat_to_array(Quat::IDENTITY),
            scale: [1.0, 1.0, 1.0],
        }
    }
}

pub struct TransformReplicator;

impl TransformReplicator {
    pub fn config(interpolation_seconds: f32) -> ReplicatedComponentConfig {
        ReplicatedComponentConfig::new(Self::component_name())
            .unreliable_ordered()
            .interpolated(interpolation_seconds)
    }


    pub fn apply_snapshot(world: &mut World, entity: Entity, snapshot: &TransformSnapshot) {
        <Self as ReplicatedComponentAdapter>::apply(world, entity, snapshot);
    }

    pub fn capture_actor(
        engine: &Engine,
        actor: Actor,
        net_id: u64,
        tick: u64,
    ) -> Option<ReplicatedComponentSnapshot<TransformSnapshot>> {
        let transform = actor.get_component::<Transform>(engine)?;
        Some(ReplicatedComponentSnapshot::new(
            net_id,
            tick,
            Self::component_name(),
            TransformSnapshot::from_transform(transform),
        ))
    }

    pub fn apply_snapshot_to_actor(engine: &mut Engine, actor: Actor, snapshot: &TransformSnapshot) {
        if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
            transform.translation = snapshot.translation_vec3();
            transform.rotation = snapshot.rotation_quat();
            transform.scale = snapshot.scale_vec3();
        }
    }

    pub fn interpolate_snapshot(from: &TransformSnapshot, to: &TransformSnapshot, alpha: f32) -> TransformSnapshot {
        <Self as ReplicatedComponentAdapter>::interpolate(from, to, alpha)
    }
}

impl ReplicatedComponentAdapter for TransformReplicator {
    type Snapshot = TransformSnapshot;

    fn component_name() -> &'static str { "transform" }

    fn capture(world: &World, entity: Entity, net_id: u64, tick: u64) -> Option<ReplicatedComponentSnapshot<Self::Snapshot>> {
        let transform = world.get::<Transform>(entity)?;
        Some(ReplicatedComponentSnapshot::new(
            net_id,
            tick,
            Self::component_name(),
            TransformSnapshot::from_transform(transform),
        ))
    }

    fn apply(world: &mut World, entity: Entity, snapshot: &Self::Snapshot) {
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            transform.translation = snapshot.translation_vec3();
            transform.rotation = snapshot.rotation_quat();
            transform.scale = snapshot.scale_vec3();
        }
    }

    fn interpolate(from: &Self::Snapshot, to: &Self::Snapshot, alpha: f32) -> Self::Snapshot {
        TransformSnapshot::interpolate(from, to, alpha)
    }
}

pub type TransformInterpolator = GenericComponentInterpolator<TransformSnapshot>;


pub fn vec3_to_array(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}

pub fn quat_to_array(value: Quat) -> [f32; 4] {
    let len_sq = value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w;
    let normalized = if len_sq > 1.0e-8 && len_sq.is_finite() { value.normalize() } else { Quat::IDENTITY };
    [normalized.x, normalized.y, normalized.z, normalized.w]
}

pub fn quat_from_array(value: [f32; 4]) -> Quat {
    let len_sq = value[0] * value[0] + value[1] * value[1] + value[2] * value[2] + value[3] * value[3];
    if len_sq > 1.0e-8 && len_sq.is_finite() {
        Quat::from_xyzw(value[0], value[1], value[2], value[3]).normalize()
    } else {
        Quat::IDENTITY
    }
}
