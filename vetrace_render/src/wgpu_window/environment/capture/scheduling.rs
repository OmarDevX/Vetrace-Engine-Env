use super::*;

impl WgpuRenderer {
    pub(super) fn render_reflection_probe_capture_work(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        scene_frame: u64,
    ) {
        let now = Instant::now();
        let mut work = Vec::new();
        for probe in frame.reflection_probes.iter().take(MAX_REFLECTION_PROBES) {
            let Some(state) = self.environment.reflection_probe_capture_states.get(&probe.entity.0) else {
                continue;
            };
            let scene_signature = reflection_probe_scene_signature(frame, probe);
            let in_progress = !matches!(state.phase, ReflectionCapturePhase::Idle);
            let due = state.transition_to.is_none()
                && state.initial_transition_started.is_none()
                && reflection_probe_capture_due(probe, state, now, scene_signature);
            if in_progress || due {
                work.push((
                    probe.entity.0,
                    in_progress,
                    probe.capture_priority.saturating_add(probe.priority),
                    probe.capture_position_world.distance_squared(frame.camera.position),
                    scene_signature,
                ));
            }
        }
        work.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.0.cmp(&b.0))
        });
        let probe_budget = frame
            .settings
            .reflection_capture_probe_budget_per_frame
            .min(MAX_REFLECTION_PROBES as u32) as usize;
        if probe_budget == 0 {
            return;
        }
        for (probe_entity, _, _, _, scene_signature) in work.into_iter().take(probe_budget) {
            let Some(probe) = frame
                .reflection_probes
                .iter()
                .find(|probe| probe.entity.0 == probe_entity)
            else {
                continue;
            };
            self.process_reflection_probe_capture_work_item(
                encoder,
                frame,
                assets,
                scene_frame,
                now,
                probe,
                scene_signature,
            );
        }
    }

    pub(super) fn process_reflection_probe_capture_work_item(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        scene_frame: u64,
        now: Instant,
        probe: &RenderReflectionProbe,
        scene_signature: u64,
    ) {
        let probe_entity = probe.entity.0;
        let Some(mut state) = self.environment.reflection_probe_capture_states.remove(&probe_entity) else {
            return;
        };
        state.update_transition(now);

        if matches!(state.phase, ReflectionCapturePhase::Idle)
            && state.transition_to.is_none()
            && state.initial_transition_started.is_none()
            && reflection_probe_capture_due(probe, &state, now, scene_signature)
        {
            let write_index = if state.has_capture { 1 - state.front_index } else { 0 };
            state.in_progress_revision = probe.capture_revision;
            state.in_progress_scene_signature = scene_signature;
            state.phase = ReflectionCapturePhase::Capturing {
                next_face: 0,
                write_index,
            };
        }

        match state.phase {
            ReflectionCapturePhase::Idle => {}
            ReflectionCapturePhase::Capturing { next_face, write_index } => {
                let face_budget = frame.settings.reflection_capture_faces_per_frame.min(6);
                if face_budget == 0 {
                    self.environment.reflection_probe_capture_states.insert(probe_entity, state);
                    return;
                }
                let end_face = (next_face + face_budget).min(6);
                for face in next_face..end_face {
                    self.render_reflection_capture_face(
                        encoder,
                        frame,
                        assets,
                        scene_frame,
                        probe,
                        &state.target,
                        face,
                    );
                    self.environment.reflection_faces_captured_this_frame =
                        self.environment.reflection_faces_captured_this_frame.saturating_add(1);
                }
                state.phase = if end_face >= 6 {
                    ReflectionCapturePhase::Filtering {
                        next_mip: 0,
                        write_index,
                    }
                } else {
                    ReflectionCapturePhase::Capturing {
                        next_face: end_face,
                        write_index,
                    }
                };
            }
            ReflectionCapturePhase::Filtering { next_mip, write_index } => {
                let mip_budget = frame
                    .settings
                    .reflection_prefilter_mips_per_frame
                    .min(ENVIRONMENT_CUBEMAP_MIP_COUNT);
                if mip_budget == 0 {
                    self.environment.reflection_probe_capture_states.insert(probe_entity, state);
                    return;
                }
                let end_mip = (next_mip + mip_budget).min(ENVIRONMENT_CUBEMAP_MIP_COUNT);
                for mip in next_mip..end_mip {
                    self.prefilter_reflection_capture_mip(
                        encoder,
                        &state.target,
                        state.slots[write_index],
                        mip,
                        frame.settings.reflection_prefilter_sample_count,
                    );
                    self.environment.reflection_mips_filtered_this_frame =
                        self.environment.reflection_mips_filtered_this_frame.saturating_add(1);
                }
                if end_mip >= ENVIRONMENT_CUBEMAP_MIP_COUNT {
                    if state.has_capture {
                        state.transition_to = Some(write_index);
                        state.transition_started = Some(now);
                    } else {
                        state.front_index = write_index;
                        state.has_capture = true;
                        state.initial_transition_started = Some(now);
                    }
                    state.completed_revision = state.in_progress_revision;
                    state.completed_scene_signature = state.in_progress_scene_signature;
                    state.last_completed = Some(now);
                    state.phase = ReflectionCapturePhase::Idle;
                } else {
                    state.phase = ReflectionCapturePhase::Filtering {
                        next_mip: end_mip,
                        write_index,
                    };
                }
            }
        }

        self.environment.reflection_probe_capture_states.insert(probe_entity, state);
    }
}
