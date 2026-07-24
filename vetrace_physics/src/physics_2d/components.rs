use glam::Vec2;
use serde::{Deserialize, Serialize};

/// How a 2D body participates in simulation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum BodyType2D {
    /// Does not move and has infinite mass.
    Static,
    /// Integrated and resolved by the physics plugin.
    #[default]
    Dynamic,
    /// Integrated from velocity but not displaced by collision impulses.
    Kinematic,
}

/// Collider primitive supported by the built-in feature-gated 2D solver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ColliderShape2D {
    Circle,
    #[default]
    Box,
}

/// Generic 2D rigid body. The authoritative pose remains the core `Transform`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigidBody2D {
    pub body_type: BodyType2D,
    pub enabled: bool,
    /// Used only by dynamic bodies. Values <= 0 are clamped to a small positive mass.
    pub mass: f32,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub lock_rotation: bool,
    /// Enables adaptive physics substeps for fast-moving bodies such as bullets.
    pub continuous: bool,
}

impl Default for RigidBody2D {
    fn default() -> Self {
        Self {
            body_type: BodyType2D::Dynamic,
            enabled: true,
            mass: 1.0,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            lock_rotation: false,
            continuous: false,
        }
    }
}

impl RigidBody2D {
    pub fn dynamic() -> Self { Self::default() }

    pub fn kinematic() -> Self {
        Self { body_type: BodyType2D::Kinematic, gravity_scale: 0.0, ..Self::default() }
    }

    pub fn static_body() -> Self {
        Self { body_type: BodyType2D::Static, gravity_scale: 0.0, ..Self::default() }
    }
}

/// Linear and angular velocity consumed and written by `Physics2dPlugin`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Velocity2D {
    pub linear: Vec2,
    /// Radians per second around the Z axis.
    pub angular: f32,
}

/// Circle or oriented-box collider attached to the entity's `Transform`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Collider2D {
    pub enabled: bool,
    pub shape: ColliderShape2D,
    /// Unscaled local half-size used by box colliders.
    pub half_extents: Vec2,
    /// Unscaled local radius used by circle colliders.
    pub radius: f32,
    pub offset: Vec2,
    /// Additional local rotation in radians for box colliders.
    pub rotation: f32,
    pub sensor: bool,
    /// At least one layer bit should normally be set.
    pub collision_layer: u32,
    /// A pair is considered only when both colliders' masks accept the other's layer.
    pub collision_mask: u32,
    pub friction: f32,
    pub restitution: f32,
}

impl Default for Collider2D {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: ColliderShape2D::Box,
            half_extents: Vec2::splat(0.5),
            radius: 0.5,
            offset: Vec2::ZERO,
            rotation: 0.0,
            sensor: false,
            collision_layer: 1,
            collision_mask: u32::MAX,
            friction: 0.4,
            restitution: 0.0,
        }
    }
}

impl Collider2D {
    pub fn circle(radius: f32) -> Self {
        Self { shape: ColliderShape2D::Circle, radius: radius.max(0.0001), ..Self::default() }
    }

    pub fn rectangle(half_extents: Vec2) -> Self {
        Self {
            shape: ColliderShape2D::Box,
            half_extents: half_extents.abs().max(Vec2::splat(0.0001)),
            ..Self::default()
        }
    }

    pub fn sensor_circle(radius: f32) -> Self {
        Self { sensor: true, ..Self::circle(radius) }
    }
}
