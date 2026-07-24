//! Serializable physics definitions used by scene/prefab files.
//!
//! Runtime physics components and Rapier handles stay owned by `vetrace_physics`.
//! Scene files store these small data definitions, then the scene loader asks
//! this crate to instantiate the actual runtime components.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use vetrace_core::{Engine, Entity};

use crate::components::{Collider, ColliderShape, KinematicBody, RigidBody3D, StaticBody};

/// Stable scene component type ID for authored physics body definitions.
pub const SCENE_PHYSICS_BODY_COMPONENT: &str = "vetrace.physics.body";
/// Stable scene component type ID for authored physics collider definitions.
pub const SCENE_PHYSICS_COLLIDER_COMPONENT: &str = "vetrace.physics.collider";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsBodyKind {
    #[default]
    Static,
    Dynamic,
    Kinematic,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhysicsBodyDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub kind: PhysicsBodyKind,
}

impl Default for PhysicsBodyDef {
    fn default() -> Self {
        Self { enabled: true, kind: PhysicsBodyKind::Static }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicsColliderShapeDef {
    /// Box/cuboid collider. Kept as `cube` in JSON for compatibility with the
    /// existing primitive names and older map-builder files.
    #[default]
    #[serde(alias = "box")]
    Cube,
    Sphere,
    Capsule,
}

impl From<ColliderShape> for PhysicsColliderShapeDef {
    fn from(value: ColliderShape) -> Self {
        match value {
            ColliderShape::Cube => Self::Cube,
            ColliderShape::Sphere => Self::Sphere,
            ColliderShape::Capsule => Self::Capsule,
        }
    }
}

impl From<PhysicsColliderShapeDef> for ColliderShape {
    fn from(value: PhysicsColliderShapeDef) -> Self {
        match value {
            PhysicsColliderShapeDef::Cube => ColliderShape::Cube,
            PhysicsColliderShapeDef::Sphere => ColliderShape::Sphere,
            PhysicsColliderShapeDef::Capsule => ColliderShape::Capsule,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhysicsColliderDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub shape: PhysicsColliderShapeDef,
    #[serde(default = "default_half_extents")]
    pub half_extents: [f32; 3],
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default)]
    pub sensor: bool,
    #[serde(default = "default_friction")]
    pub friction: f32,
    #[serde(default)]
    pub restitution: f32,
}

impl Default for PhysicsColliderDef {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: PhysicsColliderShapeDef::Cube,
            half_extents: default_half_extents(),
            offset: [0.0, 0.0, 0.0],
            sensor: false,
            friction: default_friction(),
            restitution: 0.0,
        }
    }
}

impl PhysicsColliderDef {
    pub fn from_collider(collider: &Collider) -> Self {
        Self {
            enabled: true,
            shape: collider.shape.into(),
            half_extents: collider.half_extents.to_array(),
            offset: collider.offset.to_array(),
            sensor: collider.sensor,
            friction: collider.friction,
            restitution: collider.restitution,
        }
    }

    pub fn to_runtime_collider(&self) -> Collider {
        Collider {
            handle: None,
            shape: self.shape.into(),
            half_extents: Vec3::from_array(self.half_extents).abs().max(Vec3::splat(0.001)),
            offset: Vec3::from_array(self.offset),
            sensor: self.sensor,
            friction: self.friction,
            restitution: self.restitution,
        }
    }
}

pub fn body_def_from_entity(engine: &Engine, entity: Entity) -> Option<PhysicsBodyDef> {
    if engine.raw_world().has::<StaticBody>(entity) {
        Some(PhysicsBodyDef { enabled: true, kind: PhysicsBodyKind::Static })
    } else if engine.raw_world().has::<KinematicBody>(entity) {
        Some(PhysicsBodyDef { enabled: true, kind: PhysicsBodyKind::Kinematic })
    } else if let Some(body) = engine.raw_world().get::<RigidBody3D>(entity) {
        Some(PhysicsBodyDef {
            enabled: true,
            kind: if body.dynamic { PhysicsBodyKind::Dynamic } else { PhysicsBodyKind::Kinematic },
        })
    } else {
        None
    }
}

pub fn collider_def_from_entity(engine: &Engine, entity: Entity) -> Option<PhysicsColliderDef> {
    engine.raw_world().get::<Collider>(entity).map(PhysicsColliderDef::from_collider)
}

pub fn apply_physics_defs(
    engine: &mut Engine,
    entity: Entity,
    body: Option<&PhysicsBodyDef>,
    collider: Option<&PhysicsColliderDef>,
) {
    // Reset authored body/collider marker components before applying the new
    // definition. Rapier handles are runtime-only and must not be preserved from
    // serialized data.
    let _ = engine.raw_world_mut().remove::<StaticBody>(entity);
    let _ = engine.raw_world_mut().remove::<RigidBody3D>(entity);
    let _ = engine.raw_world_mut().remove::<KinematicBody>(entity);
    let _ = engine.raw_world_mut().remove::<Collider>(entity);

    if let Some(body) = body.filter(|body| body.enabled) {
        match body.kind {
            PhysicsBodyKind::Static => {
                engine.raw_world_mut().insert(entity, StaticBody::default());
            }
            PhysicsBodyKind::Dynamic => {
                engine.raw_world_mut().insert(entity, RigidBody3D::default());
            }
            PhysicsBodyKind::Kinematic => {
                engine.raw_world_mut().insert(entity, KinematicBody::default());
            }
        }
    } else if collider.map(|collider| collider.enabled).unwrap_or(false) {
        // A collider with no explicit body is authored as a static solid by
        // default. This keeps old map-builder files working and matches the
        // usual map-geometry use case.
        engine.raw_world_mut().insert(entity, StaticBody::default());
    }

    if let Some(collider) = collider.filter(|collider| collider.enabled) {
        engine.raw_world_mut().insert(entity, collider.to_runtime_collider());
    }
}

fn default_true() -> bool { true }
fn default_half_extents() -> [f32; 3] { [0.5, 0.5, 0.5] }
fn default_friction() -> f32 { 0.7 }
