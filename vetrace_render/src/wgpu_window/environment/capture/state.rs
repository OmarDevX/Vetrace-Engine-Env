use super::*;

impl WgpuRenderer {
    pub(super) fn sync_reflection_probe_capture_states(&mut self, frame: &RenderFrame) {
        let now = Instant::now();
        let max_resident = frame
            .settings
            .reflection_max_resident_runtime_probes
            .min(MAX_REFLECTION_PROBES as u32) as usize;
        let distance_limit = frame.settings.reflection_capture_distance_limit;
        let distance_limit2 = distance_limit * distance_limit;
        let mut candidates: Vec<(u64, i32, f32, bool)> = frame
            .reflection_probes
            .iter()
            .take(MAX_REFLECTION_PROBES)
            .filter(|probe| !matches!(probe.capture_mode, crate::components::ReflectionProbeCaptureMode::Imported))
            .filter_map(|probe| {
                let distance2 = probe.capture_position_world.distance_squared(frame.camera.position);
                let already_resident = self.environment.reflection_probe_capture_states.contains_key(&probe.entity.0);
                let residency_grace = if already_resident { 1.21 } else { 1.0 };
                if distance_limit > 0.0 && distance2 > distance_limit2 * residency_grace {
                    return None;
                }
                Some((
                    probe.entity.0,
                    probe.capture_priority.saturating_add(probe.priority),
                    distance2 * if already_resident { 0.82 } else { 1.0 },
                    already_resident,
                ))
            })
            .collect();
        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.3.cmp(&a.3))
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.0.cmp(&b.0))
        });
        candidates.truncate(max_resident);
        let active_entities: HashSet<u64> = candidates.iter().map(|candidate| candidate.0).collect();

        let before_retain = self.environment.reflection_probe_capture_states.len();
        self.environment.reflection_probe_capture_states
            .retain(|entity, _| active_entities.contains(entity));
        self.environment.reflection_probe_evictions_total = self.environment.reflection_probe_evictions_total.saturating_add(
            before_retain.saturating_sub(self.environment.reflection_probe_capture_states.len()) as u64,
        );
        for state in self.environment.reflection_probe_capture_states.values_mut() {
            state.update_transition(now);
        }

        for probe in frame.reflection_probes.iter().take(MAX_REFLECTION_PROBES) {
            if !active_entities.contains(&probe.entity.0) {
                continue;
            }
            let recreate = self
                .environment
                .reflection_probe_capture_states
                .get(&probe.entity.0)
                .is_some_and(|state| {
                    state.target.resolution != probe.capture_resolution
                        && matches!(state.phase, ReflectionCapturePhase::Idle)
                        && state.transition_to.is_none()
                });
            if recreate {
                self.environment.reflection_probe_capture_states.remove(&probe.entity.0);
            }
            if self.environment.reflection_probe_capture_states.contains_key(&probe.entity.0) {
                if let Some(state) = self.environment.reflection_probe_capture_states.get_mut(&probe.entity.0) {
                    state.transition_duration = probe.transition_seconds;
                    let scene_signature = reflection_probe_scene_signature(frame, probe);
                    state.observe_scene_signature(scene_signature, now);
                }
                continue;
            }
            let Some(slots) = self.allocate_reflection_capture_slot_pair() else { continue; };
            let target = GpuReflectionCaptureTarget::new(
                &self.core.device,
                &self.scene.camera_layout,
                &self.environment.reflection_prefilter_layout,
                probe.capture_resolution,
                &format!("probe {}", probe.entity.0),
            );
            self.environment.reflection_probe_capture_states.insert(
                probe.entity.0,
                ReflectionProbeCaptureState {
                    target,
                    slots,
                    front_index: 0,
                    has_capture: false,
                    transition_to: None,
                    transition_started: None,
                    transition_duration: probe.transition_seconds,
                    initial_transition_started: None,
                    phase: ReflectionCapturePhase::Idle,
                    completed_revision: u32::MAX,
                    in_progress_revision: probe.capture_revision,
                    completed_scene_signature: u64::MAX,
                    in_progress_scene_signature: 0,
                    observed_scene_signature: reflection_probe_scene_signature(frame, probe),
                    scene_change_observed_at: Some(now),
                    last_completed: None,
                },
            );
        }
    }

    pub(super) fn allocate_reflection_capture_slot_pair(&self) -> Option<[u32; 2]> {
        let mut used = HashSet::new();
        for state in self.environment.reflection_probe_capture_states.values() {
            used.insert(state.slots[0]);
            used.insert(state.slots[1]);
        }
        for index in 0..MAX_REFLECTION_PROBES as u32 {
            let first = ENVIRONMENT_RUNTIME_SLOT_BASE + index * 2;
            let second = first + 1;
            if second < ENVIRONMENT_CUBEMAP_CAPACITY
                && !used.contains(&first)
                && !used.contains(&second)
            {
                return Some([first, second]);
            }
        }
        None
    }

    pub(super) fn captured_environment_slot_pair(
        &self,
        probe: &RenderReflectionProbe,
        now: Instant,
    ) -> Option<(u32, u32, f32)> {
        let state = self
            .environment
            .reflection_probe_capture_states
            .get(&probe.entity.0)
            .filter(|state| state.has_capture)?;
        if let Some(started) = state.initial_transition_started {
            let duration = state.transition_duration.max(0.0);
            let t = if duration <= 0.0 {
                1.0
            } else {
                (now.duration_since(started).as_secs_f32() / duration).clamp(0.0, 1.0)
            };
            let (imported_primary, imported_secondary, imported_transition) =
                self.environment_slot_pair(probe.primary, probe.secondary, probe.transition);
            let imported = if imported_transition >= 0.5 {
                imported_secondary
            } else {
                imported_primary
            };
            let dynamic = state.slots[state.front_index];
            if imported != 0 && t < 1.0 {
                return Some((imported, dynamic, t));
            }
        }
        Some(state.slot_pair(now))
    }
}
