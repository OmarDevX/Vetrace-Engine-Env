use glam::Vec3;
use rapier3d::na as nalgebra;
use rapier3d::prelude::{point, vector, PhysicsPipeline, QueryFilter, Ray, Real};
use vetrace_core::backends::{PhysicsBackend, RaycastHit};
use vetrace_core::engine::Engine;

use crate::character::{
    apply_character_body_motion, prepare_character_bodies, snap_character_bodies_to_ground,
    update_character_controller_states,
};
use crate::raycast::update_raycast_components;
use crate::state::PhysicsState;
use crate::sync::{sync_rapier_to_world, sync_world_to_rapier};

#[derive(Default)]
pub struct RapierPhysicsBackend;

impl RapierPhysicsBackend {
    pub fn new() -> Self { Self }
}

impl PhysicsBackend for RapierPhysicsBackend {
    fn step(&mut self, engine: &mut Engine, dt: f32) {
        if !engine.contains_resource::<PhysicsState>() {
            engine.insert_resource(PhysicsState::new());
        }

        let started = std::time::Instant::now();
        prepare_character_bodies(engine);
        engine.profile_record_timing("physics.prepare_character_bodies", started.elapsed());

        let started = std::time::Instant::now();
        apply_character_body_motion(engine, dt);
        engine.profile_record_timing("physics.apply_character_body_motion", started.elapsed());

        let started = std::time::Instant::now();
        sync_world_to_rapier(engine);
        engine.profile_record_timing("physics.sync_world_to_rapier", started.elapsed());

        let started = std::time::Instant::now();
        {
            let Some(state) = engine.get_resource_mut::<PhysicsState>() else { return; };
            if dt > 0.0 {
                state.integration_parameters.dt = dt.clamp(1.0 / 240.0, 1.0 / 20.0);
            }
            let mut physics_pipeline = PhysicsPipeline::new();
            physics_pipeline.step(
                &state.gravity,
                &state.integration_parameters,
                &mut state.islands,
                &mut state.broad_phase,
                &mut state.narrow_phase,
                &mut state.bodies,
                &mut state.colliders,
                &mut state.impulse_joints,
                &mut state.multibody_joints,
                &mut state.ccd_solver,
                None,
                &(),
                &(),
            );
            state.query_pipeline.update(&state.colliders);
        }
        engine.profile_record_timing("physics.rapier_pipeline_step", started.elapsed());

        let started = std::time::Instant::now();
        sync_rapier_to_world(engine);
        engine.profile_record_timing("physics.sync_rapier_to_world", started.elapsed());

        let started = std::time::Instant::now();
        update_character_controller_states(engine);
        engine.profile_record_timing("physics.update_character_controller_states", started.elapsed());

        let started = std::time::Instant::now();
        snap_character_bodies_to_ground(engine);
        engine.profile_record_timing("physics.snap_character_bodies_to_ground", started.elapsed());

        let started = std::time::Instant::now();
        update_raycast_components(engine);
        engine.profile_record_timing("physics.update_raycast_components", started.elapsed());
    }

    fn raycast(&self, engine: &Engine, origin: Vec3, dir: Vec3) -> Option<RaycastHit> {
        let state = engine.get_resource::<PhysicsState>()?;
        let dir = dir.normalize_or_zero();
        if dir.length_squared() == 0.0 { return None; }
        let ray = Ray::new(point![origin.x, origin.y, origin.z], vector![dir.x, dir.y, dir.z]);
        let hit = state.query_pipeline.cast_ray(
            &state.bodies,
            &state.colliders,
            &ray,
            Real::MAX,
            true,
            QueryFilter::default(),
        )?;
        Some(RaycastHit {
            entity: state.collider_entities.get(&hit.0).copied(),
            position: origin + dir * hit.1,
            distance: hit.1,
        })
    }
}
