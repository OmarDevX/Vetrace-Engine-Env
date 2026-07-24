use super::*;

impl WgpuRenderer {
    pub(super) fn begin_gpu_frame_profiling(&mut self) {
        #[cfg(feature = "profiler")]
        if let Some(gpu_profiler) = self.optional.gpu_timestamp_profiler.as_mut() {
            gpu_profiler.begin_frame(&self.core.device);
        } else {
            vetrace_profiler::record_counter("wgpu.gpu.timestamp_queries_enabled", 0.0, "");
        }
    }

    pub(super) fn acquire_surface_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        let texture = match self.core.surface.get_current_texture() {
            Ok(frame) => Some(frame),
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.core.surface.configure(&self.core.device, &self.core.config);
                None
            }
            Err(wgpu::SurfaceError::Timeout | wgpu::SurfaceError::OutOfMemory) => None,
        };
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.acquire_surface", started.elapsed());
        texture
    }

    pub(super) fn prepare_transient_frame_targets(
        &mut self,
        frame: &RenderFrame,
        has_shadow_light: bool,
    ) -> (bool, bool, wgpu::CommandEncoder) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();

        self.sync_render_texture_targets(frame);
        let evsm_requested = frame.settings.shadow_filter_mode == ShadowFilterMode::EvsmBlurred
            && has_shadow_light;
        self.ensure_shadow_target_size(
            frame.settings.shadow_map_size,
            frame.settings.shadow_cascade_count,
            evsm_requested,
        );

        let ssao_enabled = Self::ao_enabled_for_frame(frame);
        if ssao_enabled {
            self.ensure_ao_target_size(
                self.core.config.width,
                self.core.config.height,
                self.core.surface_view_format,
            );
        } else {
            self.release_ao_target_if_unused();
        }

        let post_process_pass_count = Self::post_process_pass_count(frame);
        let post_process_enabled = post_process_pass_count > 0;
        if post_process_enabled {
            self.ensure_post_process_targets_size(
                self.core.config.width,
                self.core.config.height,
                self.core.surface_view_format,
                post_process_pass_count > 1,
            );
        } else {
            self.release_post_process_targets_if_unused();
        }

        if Self::ssr_enabled_for_frame(frame) {
            self.ensure_ssr_history_size(
                self.core.config.width,
                self.core.config.height,
                self.core.surface_view_format,
            );
        } else {
            self.release_ssr_history_if_unused();
        }

        let encoder = self
            .core
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("vetrace wgpu render encoder"),
            });
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing(
            "wgpu.game.setup_targets_and_encoder",
            started.elapsed(),
        );
        (ssao_enabled, post_process_enabled, encoder)
    }

    pub(super) fn sync_frame_gpu_resources(
        &mut self,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
    ) -> u64 {
        self.environment.reflection_faces_captured_this_frame = 0;
        self.environment.reflection_mips_filtered_this_frame = 0;
        self.update_camera_uniform_for_frame(frame);
        self.sync_environment_for_frame(frame, assets);
        self.rebuild_reflection_probe_spatial_index(frame);
        let baked_lightmap_atlas = frame.objects.iter().find_map(|object| {
            object
                .baked_lightmap
                .as_ref()
                .map(|lightmap| lightmap.atlas.as_ref())
        });
        self.sync_baked_lightmap_atlas(baked_lightmap_atlas);
        self.scene.scene_cache_frame = self.scene.scene_cache_frame.wrapping_add(1);
        self.scene.scene_cache_frame
    }

    pub(super) fn evict_frame_caches(
        &mut self,
        scene_frame: u64,
        assets: Option<&RenderAssets>,
    ) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        self.evict_old_scene_draw_cache_entries(scene_frame);
        self.evict_old_geometry_cache_entries(scene_frame);
        self.evict_removed_texture_cache_entries(assets);
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.evict_caches", started.elapsed());
    }
}
