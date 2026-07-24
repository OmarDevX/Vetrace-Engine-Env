use std::any::Any;
use std::error::Error;

use vetrace_core::{ComponentManager, Engine, Plugin, Stage};

use super::solver::step_physics_2d;
use super::{
    BodyType2D, Collider2D, ColliderShape2D, Physics2dState, RigidBody2D, Velocity2D,
};

pub struct Physics2dPlugin;

impl Physics2dPlugin {
    pub fn new() -> Self { Self }
}

impl Default for Physics2dPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for Physics2dPlugin {
    fn name(&self) -> &'static str { "physics_2d" }
    fn update_stage(&self) -> Stage { Stage::Physics }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        if !engine.contains_resource::<Physics2dState>() {
            engine.insert_resource(Physics2dState::new());
        }
        if let Some(registry) = engine.get_resource_mut::<ComponentManager>() {
            registry.register_reflected_named::<RigidBody2D>(
                "vetrace.physics.rigid_body_2d",
                "Rigid Body 2D",
                "Physics 2D",
            );
            registry.register_reflected_named::<Collider2D>(
                "vetrace.physics.collider_2d",
                "Collider 2D",
                "Physics 2D",
            );
            registry.register_reflected_named::<Velocity2D>(
                "vetrace.physics.velocity_2d",
                "Velocity 2D",
                "Physics 2D",
            );
            publish_enum_field::<BodyType2D>(registry, "vetrace.physics.rigid_body_2d", "body_type")?;
            publish_enum_field::<ColliderShape2D>(registry, "vetrace.physics.collider_2d", "shape")?;
            let _ = registry.register_alias("vetrace.physics.rigid_body_2d", "RigidBody2D");
            let _ = registry.register_alias("vetrace.physics.collider_2d", "Collider2D");
            let _ = registry.register_alias("vetrace.physics.velocity_2d", "Velocity2D");
        }
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        if !engine.contains_resource::<Physics2dState>() {
            engine.insert_resource(Physics2dState::new());
        }
        let _ = engine.with_resource_removed::<Physics2dState, _>(|state, engine| {
            step_physics_2d(state, engine, dt);
        });
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_emits_collision_start_events() {
        use glam::{Vec2, Vec3};
        use vetrace_core::Transform;

        let mut engine = Engine::new();
        let mut plugin = Physics2dPlugin::new();
        plugin.initialize(&mut engine).unwrap();
        engine.get_resource_mut::<Physics2dState>().unwrap().gravity = Vec2::ZERO;
        engine
            .spawn_actor("dynamic")
            .with(Transform::default())
            .with(RigidBody2D::dynamic())
            .with(Collider2D::circle(0.5))
            .with(Velocity2D::default())
            .build();
        engine
            .spawn_actor("static")
            .with(Transform { translation: Vec3::new(0.8, 0.0, 0.0), ..Transform::default() })
            .with(RigidBody2D::static_body())
            .with(Collider2D::circle(0.5))
            .build();

        plugin.update(&mut engine, 1.0 / 60.0).unwrap();
        assert_eq!(engine.drain_events::<super::super::CollisionStarted2D>().len(), 1);
    }

    #[test]
    fn dynamic_body_without_velocity_gets_persistent_velocity() {
        use glam::Vec2;
        use vetrace_core::Transform;

        let mut engine = Engine::new();
        let mut plugin = Physics2dPlugin::new();
        plugin.initialize(&mut engine).unwrap();
        engine.get_resource_mut::<Physics2dState>().unwrap().gravity = Vec2::new(0.0, -10.0);
        let entity = engine
            .spawn_actor("falling")
            .with(Transform::default())
            .with(RigidBody2D::dynamic())
            .with(Collider2D::circle(0.5))
            .build()
            .entity();

        plugin.update(&mut engine, 1.0 / 60.0).unwrap();
        let velocity = engine.raw_world().get::<Velocity2D>(entity).unwrap();
        assert!(velocity.linear.y < 0.0);
    }

    #[test]
    fn plugin_publishes_2d_enum_options() {
        let mut engine = Engine::new();
        Physics2dPlugin::new().initialize(&mut engine).unwrap();
        let registry = engine.get_resource::<ComponentManager>().unwrap();
        let body_schema = registry
            .descriptor("vetrace.physics.rigid_body_2d")
            .unwrap()
            .schema
            .as_ref()
            .unwrap();
        let body_type = body_schema.fields.iter().find(|field| field.name == "body_type").unwrap();
        assert_eq!(
            body_type.enum_variants.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["Static", "Dynamic", "Kinematic"],
        );
    }
}
