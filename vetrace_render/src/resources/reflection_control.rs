use std::collections::HashSet;

use vetrace_core::Entity;

use crate::components::ReflectionProbe;

/// Thread-local command queue used by editors and gameplay tools to request
/// reflection-probe recaptures without reaching into renderer-private state.
#[derive(Clone, Debug, Default)]
pub struct ReflectionProbeCaptureRequests {
    entities: HashSet<u64>,
    capture_all: bool,
}

impl ReflectionProbeCaptureRequests {
    pub fn request(&mut self, entity: Entity) {
        self.entities.insert(entity.0);
    }

    pub fn request_all(&mut self) {
        self.capture_all = true;
    }

    pub fn is_empty(&self) -> bool {
        !self.capture_all && self.entities.is_empty()
    }

    pub(crate) fn take(&mut self) -> (bool, HashSet<u64>) {
        let capture_all = std::mem::take(&mut self.capture_all);
        let entities = std::mem::take(&mut self.entities);
        (capture_all, entities)
    }
}

/// Applies queued requests to the public probe revision. Keeping this as an
/// ordinary engine operation makes the same workflow available to Studio,
/// map-builder tools, Lua bindings, and games.
pub(crate) fn apply_reflection_probe_capture_requests(engine: &mut vetrace_core::Engine) {
    let Some((capture_all, entities)) = engine
        .get_resource_mut::<ReflectionProbeCaptureRequests>()
        .map(ReflectionProbeCaptureRequests::take)
    else {
        return;
    };
    if !capture_all && entities.is_empty() {
        return;
    }
    for (entity, probe) in engine.raw_world_mut().query_mut::<ReflectionProbe>() {
        if capture_all || entities.contains(&entity.0) {
            probe.request_capture();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vetrace_core::Engine;

    #[test]
    fn queued_request_advances_only_the_selected_probe() {
        let mut engine = Engine::new();
        engine.insert_resource(ReflectionProbeCaptureRequests::default());
        let first = engine.spawn_actor("first").with(ReflectionProbe::default()).build();
        let second = engine.spawn_actor("second").with(ReflectionProbe::default()).build();
        engine
            .get_resource_mut::<ReflectionProbeCaptureRequests>()
            .unwrap()
            .request(first.entity());
        apply_reflection_probe_capture_requests(&mut engine);
        assert_eq!(engine.raw_world().get::<ReflectionProbe>(first.entity()).unwrap().capture_revision, 1);
        assert_eq!(engine.raw_world().get::<ReflectionProbe>(second.entity()).unwrap().capture_revision, 0);
    }
}
