use glam::Vec3;
use vetrace_core::{Actor, ActorError, Bundle, Engine};

use crate::{AngularVelocity, CharacterBody3D, CharacterControllerState, Collider, RigidBody3D, Velocity};

#[derive(Clone, Debug, Default)]
pub struct RigidBodyBundle {
    pub body: RigidBody3D,
    pub collider: Collider,
    pub velocity: Velocity,
    pub angular_velocity: AngularVelocity,
}

impl Bundle for RigidBodyBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
        actor.insert(engine, self.body)?;
        actor.insert(engine, self.collider)?;
        actor.insert(engine, self.velocity)?;
        actor.insert(engine, self.angular_velocity)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct CharacterBodyBundle {
    pub body: CharacterBody3D,
    pub collider: Collider,
    pub velocity: Velocity,
    pub state: CharacterControllerState,
}

impl Bundle for CharacterBodyBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
        actor.insert(engine, self.body.clone())?;
        actor.insert(engine, self.body.sensor())?;
        actor.insert(engine, RigidBody3D::default())?;
        actor.insert(engine, self.collider)?;
        actor.insert(engine, self.velocity)?;
        actor.insert(engine, self.state)?;
        Ok(())
    }
}

pub trait PhysicsActorExt {
    fn velocity(self, engine: &Engine) -> Vec3;
    fn set_velocity(self, engine: &mut Engine, velocity: Vec3) -> Result<(), ActorError>;
    fn apply_impulse(self, engine: &mut Engine, impulse: Vec3) -> Result<(), ActorError>;
    fn request_jump(self, engine: &mut Engine) -> Result<(), ActorError>;
}

impl PhysicsActorExt for Actor {
    fn velocity(self, engine: &Engine) -> Vec3 {
        self.get_component::<Velocity>(engine).map(|velocity| velocity.linear).unwrap_or(Vec3::ZERO)
    }

    fn set_velocity(self, engine: &mut Engine, velocity: Vec3) -> Result<(), ActorError> {
        if !self.has::<Velocity>(engine) { self.insert(engine, Velocity::default())?; }
        if let Some(current) = self.get_component_mut::<Velocity>(engine) { current.linear = velocity; }
        Ok(())
    }

    fn apply_impulse(self, engine: &mut Engine, impulse: Vec3) -> Result<(), ActorError> {
        let next = self.velocity(engine) + impulse;
        self.set_velocity(engine, next)
    }

    fn request_jump(self, engine: &mut Engine) -> Result<(), ActorError> {
        if let Some(body) = self.get_component_mut::<CharacterBody3D>(engine) {
            body.jump_requested = true;
            Ok(())
        } else {
            Err(ActorError::ManagedComponent("CharacterBody3D"))
        }
    }
}
