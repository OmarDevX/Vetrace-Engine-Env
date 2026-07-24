use std::collections::BTreeMap;

use crate::Engine;

/// Ordered engine stages. Plugins and game systems can register work without
/// relying on crate insertion order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    Startup,
    PreUpdate,
    Update,
    FixedUpdate,
    Physics,
    PostPhysics,
    PostUpdate,
    RenderExtract,
    Render,
    Cleanup,
}

pub type SystemFn = Box<dyn FnMut(&mut Engine, f32) + 'static>;

pub struct ScheduledSystem {
    pub name: String,
    run: SystemFn,
}

impl ScheduledSystem {
    fn new(name: impl Into<String>, run: impl FnMut(&mut Engine, f32) + 'static) -> Self {
        Self { name: name.into(), run: Box::new(run) }
    }
}

#[derive(Default)]
pub struct Schedule {
    systems: BTreeMap<Stage, Vec<ScheduledSystem>>,
}

impl Schedule {
    pub fn add_system(
        &mut self,
        stage: Stage,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) {
        self.systems.entry(stage).or_default().push(ScheduledSystem::new(name, system));
    }

    pub fn add_system_before(
        &mut self,
        stage: Stage,
        before: &str,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) {
        let systems = self.systems.entry(stage).or_default();
        let index = systems.iter().position(|candidate| candidate.name == before).unwrap_or(systems.len());
        systems.insert(index, ScheduledSystem::new(name, system));
    }

    pub fn add_system_after(
        &mut self,
        stage: Stage,
        after: &str,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) {
        let systems = self.systems.entry(stage).or_default();
        let index = systems
            .iter()
            .position(|candidate| candidate.name == after)
            .map(|index| index + 1)
            .unwrap_or(systems.len());
        systems.insert(index, ScheduledSystem::new(name, system));
    }

    fn run_stage(&mut self, stage: Stage, engine: &mut Engine, dt: f32) {
        if let Some(systems) = self.systems.get_mut(&stage) {
            for system in systems { (system.run)(engine, dt); }
        }
    }

    fn append(&mut self, mut other: Schedule) {
        for (stage, mut systems) in std::mem::take(&mut other.systems) {
            self.systems.entry(stage).or_default().append(&mut systems);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FixedTime {
    pub timestep: f32,
    pub max_steps_per_frame: usize,
    accumulator: f32,
}

impl Default for FixedTime {
    fn default() -> Self {
        Self { timestep: 1.0 / 60.0, max_steps_per_frame: 8, accumulator: 0.0 }
    }
}

impl FixedTime {
    pub fn new(timestep: f32) -> Self { Self { timestep: timestep.max(0.000_001), ..Self::default() } }

    pub fn push_frame(&mut self, dt: f32) -> usize {
        self.accumulator += dt.max(0.0);
        let mut steps = 0usize;
        while self.accumulator + f32::EPSILON >= self.timestep && steps < self.max_steps_per_frame {
            self.accumulator -= self.timestep;
            steps += 1;
        }
        if steps == self.max_steps_per_frame && self.accumulator >= self.timestep {
            self.accumulator %= self.timestep;
        }
        steps
    }

    pub fn alpha(&self) -> f32 { (self.accumulator / self.timestep).clamp(0.0, 1.0) }
}

impl Engine {
    pub fn add_system(
        &mut self,
        stage: Stage,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) {
        if !self.contains_resource::<Schedule>() { self.insert_resource(Schedule::default()); }
        self.get_resource_mut::<Schedule>().expect("schedule inserted").add_system(stage, name, system);
    }

    pub fn add_system_before(
        &mut self,
        stage: Stage,
        before: &str,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) {
        if !self.contains_resource::<Schedule>() { self.insert_resource(Schedule::default()); }
        self.get_resource_mut::<Schedule>()
            .expect("schedule inserted")
            .add_system_before(stage, before, name, system);
    }

    pub fn add_system_after(
        &mut self,
        stage: Stage,
        after: &str,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) {
        if !self.contains_resource::<Schedule>() { self.insert_resource(Schedule::default()); }
        self.get_resource_mut::<Schedule>()
            .expect("schedule inserted")
            .add_system_after(stage, after, name, system);
    }

    pub fn run_stage(&mut self, stage: Stage, dt: f32) {
        let mut schedule = self.remove_resource::<Schedule>().unwrap_or_default();
        schedule.run_stage(stage, self, dt);
        if let Some(added_during_stage) = self.remove_resource::<Schedule>() {
            schedule.append(added_during_stage);
        }
        self.insert_resource(schedule);
        self.flush_commands();
    }

    pub fn fixed_timestep(&self) -> f32 {
        self.get_resource::<FixedTime>().copied().unwrap_or_default().timestep
    }

    pub fn fixed_alpha(&self) -> f32 {
        self.get_resource::<FixedTime>().copied().unwrap_or_default().alpha()
    }

    pub(crate) fn fixed_steps_for_frame(&mut self, dt: f32) -> (usize, f32) {
        if !self.contains_resource::<FixedTime>() { self.insert_resource(FixedTime::default()); }
        let fixed = self.get_resource_mut::<FixedTime>().expect("fixed time inserted");
        let steps = fixed.push_frame(dt);
        (steps, fixed.timestep)
    }
}
