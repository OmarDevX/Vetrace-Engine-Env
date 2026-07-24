use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ComponentInterpolationState<S> {
    pub from: S,
    pub to: S,
    pub age: f32,
    pub duration: f32,
}

impl<S> ComponentInterpolationState<S> {
    pub fn new(from: S, to: S, duration: f32) -> Self {
        Self { from, to, age: 0.0, duration: duration.max(0.001) }
    }

    pub fn advance(&mut self, dt: f32) -> InterpolationStep {
        self.age += dt.max(0.0);
        let alpha = (self.age / self.duration).clamp(0.0, 1.0);
        let smooth = alpha * alpha * (3.0 - 2.0 * alpha);
        InterpolationStep { alpha: smooth, complete: alpha >= 1.0 }
    }

    pub fn advance_cloned(&mut self, dt: f32) -> (S, S, f32, bool)
    where
        S: Clone,
    {
        let step = self.advance(dt);
        (self.from.clone(), self.to.clone(), step.alpha, step.complete)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct InterpolationStep {
    pub alpha: f32,
    pub complete: bool,
}

#[derive(Clone, Debug)]
pub struct GenericComponentInterpolator<S> {
    states: HashMap<(u64, String), ComponentInterpolationState<S>>,
}

impl<S> GenericComponentInterpolator<S> {
    pub fn new() -> Self { Self { states: HashMap::new() } }

    pub fn begin(&mut self, net_id: u64, component_name: impl Into<String>, from: S, to: S, duration: f32) {
        self.states.insert((net_id, component_name.into()), ComponentInterpolationState::new(from, to, duration));
    }

    pub fn insert(
        &mut self,
        net_id: u64,
        component_name: impl Into<String>,
        state: ComponentInterpolationState<S>,
    ) -> Option<ComponentInterpolationState<S>> {
        self.states.insert((net_id, component_name.into()), state)
    }

    pub fn get_mut(&mut self, net_id: u64, component_name: &str) -> Option<&mut ComponentInterpolationState<S>> {
        self.states.get_mut(&(net_id, component_name.to_string()))
    }

    pub fn remove(&mut self, net_id: u64, component_name: &str) -> Option<ComponentInterpolationState<S>> {
        self.states.remove(&(net_id, component_name.to_string()))
    }

    pub fn ids_for(&self, component_name: &str) -> Vec<u64> {
        self.states
            .keys()
            .filter_map(|(net_id, name)| (name == component_name).then_some(*net_id))
            .collect()
    }

    pub fn keys(&self) -> impl Iterator<Item = &(u64, String)> {
        self.states.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

impl<S> Default for GenericComponentInterpolator<S> {
    fn default() -> Self { Self::new() }
}
