use crate::components::components::{Lerp, Particle, Transform};
use crate::ecs::Entity;
use crate::{engine::engine::Engine, Behaviour};

pub struct CpuParticleSystem;

impl Default for CpuParticleSystem {
    fn default() -> Self {
        Self
    }
}

impl Behaviour for CpuParticleSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut lerp_entities = std::collections::HashSet::new();
        let mut to_remove: Vec<Entity> = Vec::new();

        {
            let mut query = engine
                .world
                .query3_mut::<Transform, Particle, Lerp>();
            for (entity, transform, particle, lerp) in query.iter_mut() {
                let Lerp::F32(inner) = lerp else { continue };
                lerp_entities.insert(*entity);
                if particle.initial_position.is_none() {
                    particle.initial_position = Some(transform.position);
                }
                transform.position[0] += particle.velocity[0] * delta;
                transform.position[1] += particle.velocity[1] * delta;
                transform.position[2] += particle.velocity[2] * delta;

                transform.size = [inner.value(); 3];
                if particle.initial_lifetime == 0.0 {
                    particle.initial_lifetime = 1.0_f32.max(inner.speed.recip());
                }
                particle.lifetime =
                    particle.initial_lifetime * (1.0 - inner.progress);

                if particle.lifetime <= 0.0 {
                    if particle.looping {
                        particle.lifetime = particle.initial_lifetime;
                        transform.size = [particle.start_size; 3];
                        if let Some(pos) = particle.initial_position {
                            transform.position = pos;
                        }
                    } else {
                        to_remove.push(*entity);
                    }
                }
            }
        }

        let mut query = engine.world.query2_mut::<Transform, Particle>();
        for (entity, transform, particle) in query.iter_mut() {
            if lerp_entities.contains(entity) {
                continue;
            }
            if particle.initial_position.is_none() {
                particle.initial_position = Some(transform.position);
            }
            transform.position[0] += particle.velocity[0] * delta;
            transform.position[1] += particle.velocity[1] * delta;
            transform.position[2] += particle.velocity[2] * delta;

            if particle.initial_lifetime == 0.0 {
                particle.initial_lifetime = particle.lifetime.max(0.0001);
            }
            let progress = 1.0 - (particle.lifetime / particle.initial_lifetime);
            let size = particle.start_size + (particle.end_size - particle.start_size) * progress;
            transform.size = [size, size, size];
            particle.lifetime -= delta;

            if particle.lifetime <= 0.0 {
                if particle.looping {
                    particle.lifetime = particle.initial_lifetime;
                    transform.size = [particle.start_size; 3];
                    if let Some(pos) = particle.initial_position {
                        transform.position = pos;
                    }
                } else {
                    to_remove.push(*entity);
                }
            }
        }
        for e in to_remove {
            engine.world.remove::<Particle>(e);
            engine.world.remove::<Transform>(e);
            engine.world.delete_entity(e);
        }
    }
}