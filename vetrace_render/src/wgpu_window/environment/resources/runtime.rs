use super::*;

impl WgpuRenderer {
    pub(super) fn sync_environment_for_frame(&mut self, frame: &RenderFrame, assets: Option<&RenderAssets>) {
        self.sync_reflection_probe_capture_states(frame);
        let now = Instant::now();
        // Preserve semantic priority instead of sorting by arbitrary handle ID:
        // global transition maps first, then already-priority-sorted local probes.
        let mut handles = Vec::new();
        let mut append_handle = |handle: Option<crate::components::CubemapHandle>| {
            if let Some(handle) = handle {
                if !handles.contains(&handle) && handles.len() < ENVIRONMENT_STATIC_SLOT_COUNT as usize {
                    handles.push(handle);
                }
            }
        };
        if let Some(environment) = &frame.environment {
            append_handle(environment.primary);
            append_handle(environment.secondary);
        }
        for probe in frame.reflection_probes.iter().take(MAX_REFLECTION_PROBES) {
            append_handle(probe.primary);
            append_handle(probe.secondary);
        }

        let signature = environment_assets_signature(&handles, assets);
        if signature != self.environment.environment_cubemap_pool.signature {
            self.environment.environment_cubemap_pool.slots.clear();
            if let Some(assets) = assets {
                for (index, handle) in handles.iter().enumerate() {
                    let slot = index as u32 + 1;
                    if let Some(asset) = assets.cubemaps.get(&handle.0).filter(|asset| asset.is_valid()) {
                        upload_cubemap_asset(
                            &self.core.queue,
                            &self.environment.environment_cubemap_pool.texture,
                            slot,
                            asset,
                        );
                        self.environment.environment_cubemap_pool.slots.insert(handle.0, slot);
                    }
                }
            }
            self.environment.environment_cubemap_pool.signature = signature;
        }

        let mut gpu_probes = [GpuReflectionProbe::zeroed(); MAX_REFLECTION_PROBES];
        let probe_count = frame.reflection_probes.len().min(MAX_REFLECTION_PROBES);
        for (gpu, probe) in gpu_probes.iter_mut().zip(frame.reflection_probes.iter()).take(probe_count) {
            let (probe_primary_slot, probe_secondary_slot, probe_transition) = self
                .captured_environment_slot_pair(probe, now)
                .unwrap_or_else(|| self.environment_slot_pair(probe.primary, probe.secondary, probe.transition));
            *gpu = GpuReflectionProbe {
                world_to_probe: probe.world_to_probe.to_cols_array_2d(),
                half_extents_blend: [
                    probe.half_extents.x,
                    probe.half_extents.y,
                    probe.half_extents.z,
                    probe.blend_distance,
                ],
                capture_intensity: [
                    probe.capture_position_local.x,
                    probe.capture_position_local.y,
                    probe.capture_position_local.z,
                    probe.intensity,
                ],
                slots_modes: {
                    [
                        probe_primary_slot,
                        probe_secondary_slot,
                        match probe.parallax_mode {
                            crate::components::ReflectionProbeParallaxMode::Disabled => 0,
                            crate::components::ReflectionProbeParallaxMode::BoxProjection => 1,
                        },
                        probe.priority as u32,
                    ]
                },
                transition_params: [probe_transition, 0.0, 0.0, 0.0],
                layer_masks: [probe.include_layers, probe.exclude_layers, 0, 0],
            };
        }
        self.core.queue.write_buffer(&self.environment.reflection_probe_buffer, 0, bytemuck::cast_slice(&gpu_probes));

        let environment = frame.environment.as_ref();
        let (global_primary_slot, global_secondary_slot, global_transition) = environment
            .map(|value| self.environment_slot_pair(value.primary, value.secondary, value.transition))
            .unwrap_or((0, 0, 0.0));
        let uniform = EnvironmentUniform {
            slots_counts: [
                global_primary_slot,
                global_secondary_slot,
                probe_count as u32,
                ENVIRONMENT_CUBEMAP_MIP_COUNT,
            ],
            params0: [
                global_transition,
                environment.map_or(0.0, |value| value.intensity),
                environment.map_or(0.0, |value| value.rotation_radians),
                if environment.is_some() { 1.0 } else { 0.0 },
            ],
            params1: [
                if environment.is_some_and(|value| value.draw_sky) { 1.0 } else { 0.0 },
                if environment.is_some_and(|value| value.diffuse_ibl) { 1.0 } else { 0.0 },
                if environment.is_some_and(|value| value.specular_ibl) { 1.0 } else { 0.0 },
                0.0,
            ],
            post_process: [
                frame.post_processing.exposure.max(0.0001),
                frame.post_processing.gamma.clamp(1.0, 3.0),
                frame.post_processing.tone_mapper.shader_value(),
                0.0,
            ],
        };
        self.core.queue.write_buffer(&self.environment.environment_uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        let capture_uniform = EnvironmentUniform {
            slots_counts: [
                global_primary_slot,
                global_secondary_slot,
                0,
                ENVIRONMENT_CUBEMAP_MIP_COUNT,
            ],
            params0: uniform.params0,
            params1: [uniform.params1[0], uniform.params1[1], uniform.params1[2], 1.0],
            post_process: uniform.post_process,
        };
        self.core.queue.write_buffer(
            &self.environment.capture_environment_uniform_buffer,
            0,
            bytemuck::bytes_of(&capture_uniform),
        );
        self.environment.environment_sky_enabled = uniform.params0[3] >= 0.5
            && uniform.params1[0] >= 0.5
            && (uniform.slots_counts[0] != 0 || uniform.slots_counts[1] != 0);
        self.environment.capture_sky_enabled = capture_uniform.params0[3] >= 0.5
            && capture_uniform.params1[0] >= 0.5
            && (capture_uniform.slots_counts[0] != 0 || capture_uniform.slots_counts[1] != 0);
    }

    pub(super) fn environment_slot(&self, handle: Option<crate::components::CubemapHandle>) -> u32 {
        handle
            .and_then(|handle| self.environment.environment_cubemap_pool.slots.get(&handle.0).copied())
            .unwrap_or(0)
    }

    pub(super) fn environment_slot_pair(
        &self,
        primary: Option<crate::components::CubemapHandle>,
        secondary: Option<crate::components::CubemapHandle>,
        transition: f32,
    ) -> (u32, u32, f32) {
        let primary = self.environment_slot(primary);
        let secondary = self.environment_slot(secondary);
        match (primary, secondary) {
            (0, 0) => (0, 0, 0.0),
            (0, secondary) => (secondary, secondary, 0.0),
            (primary, 0) => (primary, primary, 0.0),
            (primary, secondary) => (primary, secondary, transition.clamp(0.0, 1.0)),
        }
    }
}
