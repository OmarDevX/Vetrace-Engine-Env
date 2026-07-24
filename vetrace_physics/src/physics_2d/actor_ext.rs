use glam::Vec2;
use vetrace_core::{Actor, ActorError, Bundle, Engine};

use super::{Collider2D, RigidBody2D, Velocity2D};

#[derive(Clone, Debug, Default)]
pub struct RigidBody2dBundle {
    pub body: RigidBody2D,
    pub collider: Collider2D,
    pub velocity: Velocity2D,
}

impl Bundle for RigidBody2dBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
        actor.insert(engine, self.body)?;
        actor.insert(engine, self.collider)?;
        actor.insert(engine, self.velocity)?;
        Ok(())
    }
}

pub trait Physics2dActorExt {
    fn velocity_2d(self, engine: &Engine) -> Vec2;
    fn set_velocity_2d(self, engine: &mut Engine, velocity: Vec2) -> Result<(), ActorError>;
    fn apply_impulse_2d(self, engine: &mut Engine, impulse: Vec2) -> Result<(), ActorError>;
}

impl Physics2dActorExt for Actor {
    fn velocity_2d(self, engine: &Engine) -> Vec2 {
        self.get_component::<Velocity2D>(engine)
            .map(|velocity| velocity.linear)
            .unwrap_or(Vec2::ZERO)
    }

    fn set_velocity_2d(self, engine: &mut Engine, velocity: Vec2) -> Result<(), ActorError> {
        if !self.has::<Velocity2D>(engine) { self.insert(engine, Velocity2D::default())?; }
        if let Some(current) = self.get_component_mut::<Velocity2D>(engine) {
            current.linear = velocity;
        }
        Ok(())
    }

    fn apply_impulse_2d(self, engine: &mut Engine, impulse: Vec2) -> Result<(), ActorError> {
        let mass = self
            .get_component::<RigidBody2D>(engine)
            .map(|body| body.mass.max(0.0001))
            .unwrap_or(1.0);
        let velocity = self.velocity_2d(engine) + impulse / mass;
        self.set_velocity_2d(engine, velocity)
    }
}
