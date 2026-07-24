use glam::Vec3;

use crate::{ecs::Entity, engine::Engine};

#[derive(Clone, Copy, Debug, Default)]
pub struct RaycastHit {
    pub entity: Option<Entity>,
    pub position: Vec3,
    pub distance: f32,
}

/// Engine-neutral profiling sink.
///
/// `vetrace_core` exposes only this tiny hook so crates can report timings,
/// counters, and memory estimates without depending on a concrete profiler
/// implementation. `vetrace_profiler` provides the default implementation.
pub trait ProfilerBackend: 'static {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn record_timing(&mut self, _name: &str, _duration: std::time::Duration) {}
    fn record_counter(&mut self, _name: &str, _value: f64, _unit: &'static str) {}
}

pub trait ScriptingBackend: 'static {
    fn attach_script(&mut self, engine: &mut Engine, entity: Entity, source: &str);
    fn on_update(&mut self, engine: &mut Engine, dt: f32);
}

pub trait PhysicsBackend: 'static {
    fn step(&mut self, engine: &mut Engine, dt: f32);
    fn raycast(&self, engine: &Engine, origin: Vec3, dir: Vec3) -> Option<RaycastHit>;
}

pub trait NetBackend: 'static {
    fn poll(&mut self, engine: &mut Engine);
    fn send_state(&mut self, engine: &Engine);
}

pub trait RenderBackend: 'static {
    fn render(&mut self, engine: &mut Engine);
}
