use std::any::Any;
use std::error::Error;

use glam::Vec3;
use vetrace_core::app::Plugin;
use vetrace_core::backends::PhysicsBackend;
use vetrace_core::components::builtins::Transform;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::ecs::Entity;
use vetrace_core::Stage;

use crate::backend::RapierPhysicsBackend;
use crate::components::{
    AngularVelocity, BallJoint, CharacterBody3D, CharacterController3D, CharacterControllerState,
    Collider, ColliderShape, GltfCollisionApplied, KinematicBody, MeshCollider, MeshColliderShape,
    Raycast, RevoluteJoint, RigidBody3D, StaticBody,
    Velocity,
};
use crate::state::PhysicsState;

pub struct RapierPhysicsPlugin;

impl RapierPhysicsPlugin {
    pub fn new() -> Self { Self }
}

impl Default for RapierPhysicsPlugin {
    fn default() -> Self { Self::new() }
}

fn publish_enum_field<E: vetrace_core::VetraceEnum>(
    registry: &mut ComponentManager,
    component: &str,
    path: &str,
) -> Result<(), Box<dyn Error>> {
    registry
        .register_enum_field::<E>(component, path)
        .map_err(|error| -> Box<dyn Error> {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })
}

impl Plugin for RapierPhysicsPlugin {
    fn name(&self) -> &'static str { "rapier_physics" }
    fn update_stage(&self) -> Stage { Stage::Physics }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource::<Box<dyn PhysicsBackend>>(Box::new(RapierPhysicsBackend::new()));
        engine.insert_resource(PhysicsState::new());
        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_reflected_transient_named::<RigidBody3D>("vetrace.physics.rigid_body_3d", "Rigid Body 3D", "Physics");
            cm.register_reflected_transient_named::<StaticBody>("vetrace.physics.static_body", "Static Body", "Physics");
            cm.register_reflected_transient_named::<KinematicBody>("vetrace.physics.kinematic_body", "Kinematic Body", "Physics");
            cm.register_reflected_named::<Collider>("vetrace.physics.collider", "Collider", "Physics");
            cm.register_reflected_named::<MeshCollider>("vetrace.physics.mesh_collider", "Mesh Collider", "Physics");
            cm.register_named::<GltfCollisionApplied>("vetrace.physics.gltf_collision_applied", "GLTF Collision Applied");
            cm.register_reflected_named::<RevoluteJoint>("vetrace.physics.revolute_joint", "Revolute Joint", "Physics");
            cm.register_reflected_named::<BallJoint>("vetrace.physics.ball_joint", "Ball Joint", "Physics");
            cm.register_reflected_named::<Velocity>("vetrace.physics.velocity", "Velocity", "Physics");
            cm.register_reflected_named::<AngularVelocity>("vetrace.physics.angular_velocity", "Angular Velocity", "Physics");
            cm.register_reflected_named::<Raycast>("vetrace.physics.raycast", "Raycast", "Physics");
            cm.register_reflected::<CharacterBody3D>();
            cm.register_reflected_named::<CharacterController3D>("vetrace.physics.character_controller_3d", "Character Controller 3D", "Physics");
            cm.register_serializable_readonly_transient::<CharacterControllerState>("vetrace.physics.character_controller_state", "Character Controller State");
            publish_enum_field::<ColliderShape>(cm, "vetrace.physics.collider", "shape")?;
            publish_enum_field::<MeshColliderShape>(cm, "vetrace.physics.mesh_collider", "shape")?;
            let _ = cm.register_alias("vetrace.physics.character_body_3d", "CharacterBody3D");
            let _ = cm.register_alias("vetrace.physics.rigid_body_3d", "RigidBody3D");
            let _ = cm.register_alias("vetrace.physics.collider", "Collider");
        }
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        #[cfg(feature = "gltf_collisions")]
        crate::gltf_collisions::apply_gltf_imported_colliders(engine);

        let _ = engine.with_resource_removed::<Box<dyn PhysicsBackend>, _>(
            |backend, engine| backend.step(engine, dt),
        );
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Helper for Rust apps that want to attach a simple physics body.
pub fn add_dynamic_box(engine: &mut Engine, entity: Entity, half_extents: Vec3) {
    if !engine.raw_world().has::<Transform>(entity) {
        engine.raw_world_mut().insert(entity, Transform::default());
    }
    engine.raw_world_mut().insert(entity, RigidBody3D::default());
    engine.raw_world_mut().insert(entity, Collider { handle: None, shape: ColliderShape::Cube, half_extents, offset: Vec3::ZERO, ..Collider::default() });
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_plugin_publishes_collider_enum_options() {
        let mut engine = Engine::new();
        RapierPhysicsPlugin::new().initialize(&mut engine).unwrap();
        let registry = engine.get_resource::<ComponentManager>().unwrap();
        let schema = registry
            .descriptor("vetrace.physics.collider")
            .unwrap()
            .schema
            .as_ref()
            .unwrap();
        let shape = schema.fields.iter().find(|field| field.name == "shape").unwrap();

        assert_eq!(
            shape.enum_variants.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["Sphere", "Cube", "Capsule"],
        );
    }
}
