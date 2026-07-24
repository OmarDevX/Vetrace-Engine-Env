use std::collections::HashMap;

use glam::Vec3;
use rapier3d::na as nalgebra;
use rapier3d::na::{Isometry3, Quaternion, Translation3, UnitQuaternion};
use rapier3d::prelude::{
    vector, CCDSolver, ColliderHandle, ColliderSet, DefaultBroadPhase, ImpulseJointSet,
    IntegrationParameters, IslandManager, MultibodyJointSet, NarrowPhase, QueryPipeline, Real,
    RigidBody, RigidBodyHandle, RigidBodySet, Vector,
};
use vetrace_core::components::builtins::Transform;
use vetrace_core::ecs::Entity;

use crate::colliders::ColliderSignature;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PhysicsPose {
    pub(crate) translation: Vec3,
    pub(crate) rotation: glam::Quat,
}

impl PhysicsPose {
    pub(crate) fn from_transform(transform: &Transform) -> Self {
        Self {
            translation: transform.translation,
            rotation: transform.rotation.normalize(),
        }
    }
}

pub(crate) const TRANSFORM_SYNC_TRANSLATION_EPSILON: f32 = 0.0001;
pub(crate) const TRANSFORM_SYNC_ROTATION_EPSILON: f32 = 0.00001;

pub struct PhysicsState {
    pub gravity: Vector<Real>,
    pub integration_parameters: IntegrationParameters,
    pub islands: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub entity_bodies: HashMap<Entity, RigidBodyHandle>,
    pub entity_colliders: HashMap<Entity, ColliderHandle>,
    pub collider_entities: HashMap<ColliderHandle, Entity>,
    /// Last transform pose written through the physics bridge. If the ECS
    /// Transform differs from this before the next physics step, it means
    /// gameplay, editor, networking, or scene loading intentionally moved the
    /// entity, so the bridge pushes that Transform into Rapier automatically.
    pub(crate) transform_cache: HashMap<Entity, PhysicsPose>,
    /// Last Rapier collider shape created for an entity. Collider dimensions
    /// include ECS transform scale because Rapier colliders are not scaled by
    /// parent body transforms.
    pub(crate) collider_cache: HashMap<Entity, ColliderSignature>,
}

impl PhysicsState {
    pub fn new() -> Self {
        Self {
            gravity: vector![0.0, -9.81, 0.0],
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
            entity_bodies: HashMap::new(),
            entity_colliders: HashMap::new(),
            collider_entities: HashMap::new(),
            transform_cache: HashMap::new(),
            collider_cache: HashMap::new(),
        }
    }
}

impl Default for PhysicsState {
    fn default() -> Self { Self::new() }
}

pub(crate) fn isometry_from_pose(pose: PhysicsPose) -> Isometry3<Real> {
    let rotation = pose.rotation.normalize();
    Isometry3::from_parts(
        Translation3::new(pose.translation.x, pose.translation.y, pose.translation.z),
        UnitQuaternion::from_quaternion(Quaternion::new(rotation.w, rotation.x, rotation.y, rotation.z)),
    )
}

pub(crate) fn pose_from_body(body: &RigidBody) -> PhysicsPose {
    let translation = body.translation();
    let rotation = body.rotation();
    let q = rotation.quaternion();
    PhysicsPose {
        translation: Vec3::new(translation.x, translation.y, translation.z),
        rotation: glam::Quat::from_xyzw(q.i, q.j, q.k, q.w).normalize(),
    }
}

pub(crate) fn transform_changed_externally(previous: Option<PhysicsPose>, current: PhysicsPose) -> bool {
    let Some(previous) = previous else { return true; };
    let translation_changed = previous.translation.distance_squared(current.translation)
        > TRANSFORM_SYNC_TRANSLATION_EPSILON * TRANSFORM_SYNC_TRANSLATION_EPSILON;
    let rotation_dot = previous.rotation.dot(current.rotation).abs().clamp(0.0, 1.0);
    let rotation_changed = 1.0 - rotation_dot > TRANSFORM_SYNC_ROTATION_EPSILON;
    translation_changed || rotation_changed
}
