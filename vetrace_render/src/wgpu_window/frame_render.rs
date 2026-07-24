use super::*;

// Main frame orchestration for WgpuRenderTarget.

impl WgpuRenderer {
    pub fn render_frame(&mut self, frame: &RenderFrame, assets: Option<&RenderAssets>) -> bool {
        #[cfg(feature = "profiler")]
        let wgpu_frame_started = Instant::now();
        self.begin_gpu_frame_profiling();
        let Some(surface_texture) = self.acquire_surface_texture() else { return false; };
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("vetrace surface render view"),
            // Use the surface texture's configured format. Requesting an
            // alternate view format here is unnecessary now that pipelines
            // target `surface_view_format == config.format`, and avoiding it
            // keeps current Chrome and Firefox WebGPU implementations on the
            // well-tested canvas path.
            format: None,
            ..Default::default()
        });
        let clear = clear_color_for_frame(frame);
        let shadow_light = primary_shadow_light(frame);
        let (ssao_enabled, post_process_enabled, mut encoder) =
            self.prepare_transient_frame_targets(frame, shadow_light.is_some());
        let scene_frame = self.sync_frame_gpu_resources(frame, assets);
        let shadow_vertex_limit = frame.settings.shadow_max_vertices as usize;

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        let pending_draws = self.prepare_pending_draws_for_frame(frame, assets);
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.prepare_pending_draws", started.elapsed());
        #[cfg(feature = "profiler")]
        let pending_draw_count = pending_draws.len();

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        let shadow_candidates = build_directional_shadow_candidates(
            &pending_draws,
            shadow_light.is_some(),
            shadow_vertex_limit,
            frame.camera.position,
            frame.settings.shadow_max_distance,
        );
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.select_shadow_candidates", started.elapsed());

        let shadow_info = shadow_info_for_frame(frame, shadow_light, &shadow_candidates);
        // Reflection captures may render their own camera-relative shadow maps.
        // Encode them first, then restore the main-camera shadow target before
        // render textures and the visible scene sample it.
        self.render_reflection_probe_capture_work(
            &mut encoder,
            frame,
            assets,
            scene_frame,
        );
        self.render_directional_shadow_passes(&mut encoder, frame, assets, &pending_draws, &shadow_info, &shadow_candidates);
        self.render_texture_views_for_frame(
            &mut encoder,
            frame,
            assets,
            &shadow_info,
            scene_frame,
        );

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        let PreparedSceneDraws { opaque_draws, mut transparent_draws, mut overlay_draws, outline_draws } =
            self.prepare_scene_draws_for_frame(frame, assets, pending_draws, &shadow_info, scene_frame);
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.prepare_scene_draws", started.elapsed());

        let scene_output = if ssao_enabled {
            SceneOutputTarget::SsaoSceneColor
        } else if post_process_enabled {
            SceneOutputTarget::PostProcessTargetA
        } else {
            SceneOutputTarget::Surface(&view)
        };

        self.render_scene_draws(&mut encoder, scene_output, clear, &opaque_draws, &mut transparent_draws, &mut overlay_draws);
        self.render_outline_draws(&mut encoder, scene_output, &outline_draws);

        self.evict_frame_caches(scene_frame, assets);

        if ssao_enabled {
            #[cfg(feature = "profiler")]
            let ssao_started = Instant::now();
            let ssao_output = if post_process_enabled {
                SsaoCompositeOutput::PostProcessTargetA
            } else {
                SsaoCompositeOutput::Surface(&view)
            };
            self.run_ssao_and_composite(&mut encoder, frame, ssao_output);
            #[cfg(feature = "profiler")]
            vetrace_profiler::record_timing("wgpu.game.ssao_cpu_encode", ssao_started.elapsed());
        }

        if post_process_enabled {
            #[cfg(feature = "profiler")]
            let post_process_started = Instant::now();
            self.run_post_process_chain(&mut encoder, frame, assets, &view);
            #[cfg(feature = "profiler")]
            vetrace_profiler::record_timing("wgpu.game.post_process_cpu_encode", post_process_started.elapsed());
        }

        #[cfg(feature = "render_2d")]
        self.render_canvas_2d(frame, assets, &mut encoder, &view);

        self.render_screen_overlays_and_egui(frame, &mut encoder, &view);

        #[cfg(feature = "profiler")]
        self.record_profiler_counters(
            frame,
            pending_draw_count,
            shadow_candidates.len(),
            opaque_draws.len(),
            transparent_draws.len() + overlay_draws.len(),
            outline_draws.len(),
        );

        #[cfg(feature = "profiler")]
        if let Some(gpu_profiler) = self.optional.gpu_timestamp_profiler.as_mut() {
            gpu_profiler.finish_encoder(&mut encoder);
        }

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        self.core.queue.submit(Some(encoder.finish()));
        #[cfg(feature = "profiler")]
        {
            if let Some(gpu_profiler) = self.optional.gpu_timestamp_profiler.as_mut() {
                gpu_profiler.after_submit();
            }
            vetrace_profiler::record_timing("wgpu.game.queue_submit", started.elapsed());
        }

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        surface_texture.present();
        #[cfg(feature = "profiler")]
        {
            vetrace_profiler::record_timing("wgpu.game.present", started.elapsed());
            vetrace_profiler::record_timing("wgpu.game.frame_cpu_total", wgpu_frame_started.elapsed());
        }

        #[cfg(all(feature = "egui_render", feature = "profiler", feature = "wgpu_window"))]
        self.render_detached_profiler_window(frame);
        true
    }

    pub(super) fn update_camera_uniform_for_frame(&self, frame: &RenderFrame) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        let camera_uniform = camera_uniform_for(&frame.camera, self.core.config.width, self.core.config.height);
        self.core.queue.write_buffer(&self.scene.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.update_camera_uniform", started.elapsed());
    }
}
