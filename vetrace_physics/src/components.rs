use glam::Vec3;
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use serde::{Deserialize, Serialize};
use vetrace_core::backends::RaycastHit;
use vetrace_core::ecs::Entity;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigidBody3D {
    #[serde(skip)]
    pub handle: Option<RigidBodyHandle>,
    pub dynamic: bool,
}

impl Default for RigidBody3D {
    fn default() -> Self {
        Self { handle: None, dynamic: true }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StaticBody {
    #[serde(skip)]
    pub handle: Option<RigidBodyHandle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ColliderShape {
    Sphere,
    Cube,
    Capsule,
}

impl Default for ColliderShape {
    fn default() -> Self { Self::Cube }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KinematicBody {
    #[serde(skip)]
    pub handle: Option<RigidBodyHandle>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevoluteJoint {
    pub target: Option<Entity>,
    pub axis: Vec3,
    pub limits: Option<[f32; 2]>,
}

impl Default for RevoluteJoint {
    fn default() -> Self {
        Self { target: None, axis: Vec3::Y, limits: None }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BallJoint {
    pub target: Option<Entity>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Velocity {
    pub linear: Vec3,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AngularVelocity {
    pub angular: Vec3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterController3D {
    pub radius: f32,
    pub height: f32,
    /// Maximum vertical obstacle the helper controller may step over. The
    /// active helper exposes this value and uses it for ground snap distance;
    /// game/runtime code still decides desired motion.
    pub step_height: f32,
    /// Extra distance below the capsule that is considered grounded.
    pub ground_snap: f32,
    /// Maximum slope angle accepted as walkable.
    pub max_slope_radians: f32,
}

impl CharacterController3D {
    pub fn fps_capsule(radius: f32, height: f32) -> Self {
        Self {
            radius,
            height,
            step_height: 0.35,
            ground_snap: 0.18,
            max_slope_radians: 50.0_f32.to_radians(),
        }
    }
}

impl Default for CharacterController3D {
    fn default() -> Self { Self::fps_capsule(0.45, 1.8) }
}

/// Generic Godot-style 3D character body.
///
/// This is an opt-in reusable physics component: the physics plugin will only
/// apply character movement, jump gating, slope projection and ground snapping
/// to entities that explicitly have this component. Games set the desired
/// velocity/jump intent; the physics plugin performs the generic body motion.
#[derive(Clone, Debug, Serialize, Deserialize, vetrace_core::VetraceComponent)]
#[vetrace_component(
    id = "vetrace.physics.character_body_3d",
    display_name = "Character Body 3D",
    category = "Physics"
)]
pub struct CharacterBody3D {
    #[vetrace(min = 0.01)]
    pub radius: f32,
    #[vetrace(min = 0.01)]
    pub height: f32,
    #[vetrace(min = 0.0)]
    pub move_speed: f32,
    #[vetrace(min = 0.0)]
    pub jump_speed: f32,
    #[vetrace(min = 0.0)]
    pub step_height: f32,
    #[vetrace(min = 0.0)]
    pub ground_snap: f32,
    #[vetrace(min = 0.0)]
    pub max_slope_radians: f32,
    #[vetrace(min = 0.0)]
    pub max_fall_speed: f32,
    /// Desired world-space horizontal velocity supplied by the game/app.
    #[vetrace(runtime_only)]
    pub desired_velocity: Vec3,
    /// One-frame jump request supplied by the game/app. Cleared by physics.
    #[vetrace(runtime_only)]
    pub jump_requested: bool,
    /// Whether grounded characters should be raised out of small penetration.
    pub snap_to_ground: bool,
    /// Whether grounded downward velocity should be clamped to zero.
    pub stop_on_ground: bool,
}

impl CharacterBody3D {
    pub fn capsule(radius: f32, height: f32) -> Self {
        Self {
            radius,
            height,
            move_speed: 4.5,
            jump_speed: 5.0,
            step_height: 0.35,
            ground_snap: 0.18,
            max_slope_radians: 50.0_f32.to_radians(),
            max_fall_speed: 35.0,
            desired_velocity: Vec3::ZERO,
            jump_requested: false,
            snap_to_ground: true,
            stop_on_ground: true,
        }
    }

    pub fn fps_capsule(radius: f32, height: f32) -> Self {
        Self::capsule(radius, height)
    }

    pub fn sensor(&self) -> CharacterController3D {
        CharacterController3D {
            radius: self.radius,
            height: self.height,
            step_height: self.step_height,
            ground_snap: self.ground_snap,
            max_slope_radians: self.max_slope_radians,
        }
    }
}

impl Default for CharacterBody3D {
    fn default() -> Self { Self::capsule(0.45, 1.8) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterControllerState {
    pub grounded: bool,
    pub ground_entity: Option<Entity>,
    pub ground_distance: f32,
    pub ground_normal: Vec3,
    pub vertical_speed: f32,
    pub slope_radians: f32,
}

impl Default for CharacterControllerState {
    fn default() -> Self {
        Self {
            grounded: false,
            ground_entity: None,
            ground_distance: f32::MAX,
            ground_normal: Vec3::Y,
            vertical_speed: 0.0,
            slope_radians: 0.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Raycast {
    pub origin: Vec3,
    pub direction: Vec3,
    pub max_distance: f32,
    #[serde(skip)]
    pub hit: Option<RaycastHit>,
}

impl Default for Raycast {
    fn default() -> Self {
        Self { origin: Vec3::ZERO, direction: Vec3::Z, max_distance: f32::MAX, hit: None }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Collider {
    #[serde(skip)]
    pub handle: Option<ColliderHandle>,
    pub shape: ColliderShape,
    pub half_extents: Vec3,
    pub offset: Vec3,
    pub sensor: bool,
    pub friction: f32,
    pub restitution: f32,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            handle: None,
            shape: ColliderShape::Cube,
            half_extents: Vec3::splat(0.5),
            offset: Vec3::ZERO,
            sensor: false,
            friction: 0.7,
            restitution: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum MeshColliderShape {
    /// Static triangle mesh collision. Best for level geometry; avoid on dynamic
    /// rigid bodies because it is heavy and can be unstable.
    #[default]
    TriangleMesh,
    /// Convex hull generated from the supplied points. Better for moving props.
    ConvexHull,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshCollider {
    #[serde(skip)]
    pub handle: Option<ColliderHandle>,
    pub shape: MeshColliderShape,
    pub vertices: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
    pub offset: Vec3,
    pub sensor: bool,
    pub friction: f32,
    pub restitution: f32,
}

impl Default for MeshCollider {
    fn default() -> Self {
        Self {
            handle: None,
            shape: MeshColliderShape::TriangleMesh,
            vertices: Vec::new(),
            indices: Vec::new(),
            offset: Vec3::ZERO,
            sensor: false,
            friction: 0.7,
            restitution: 0.0,
        }
    }
}

/// Marker added after a renderer-neutral `GltfImportedCollider` has been
/// converted into runtime physics components. This prevents the physics plugin
/// from overwriting edited bodies every frame.
#[derive(Clone, Debug, Default)]
pub struct GltfCollisionApplied;
