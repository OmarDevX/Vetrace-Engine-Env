use super::*;

// Directional shadow rendering for the WGPU frame path.

impl WgpuRenderer {
    pub(super) fn render_directional_shadow_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        pending_draws: &[PendingDraw<'_>],
        shadow_info: &ShadowInfo,
        shadow_candidates: &[ShadowCandidate],
    ) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        if shadow_info.enabled && !shadow_candidates.is_empty() {
            self.shadows.shadow_cache_frame = self.shadows.shadow_cache_frame.wrapping_add(1);
            let shadow_frame = self.shadows.shadow_cache_frame;

            // Resolve/create cached buffers and alpha-test material bind groups
            // before opening render passes. The pass borrows shadow layer views,
            // so mutating texture/material caches inside the pass would fail the
            // borrow checker and can also invalidate bindings on some backends.
            let mut shadow_draws: Vec<PreparedShadowDraw> = Vec::with_capacity(shadow_candidates.len());
            for candidate in shadow_candidates {
                let pending = &pending_draws[candidate.index];
                let buffers = self.geometry_buffers_for(pending.geometry_key, pending.geometry_signature, &pending.geometry, shadow_frame);
                if buffers.draw_count() > 0 {
                    shadow_draws.push(self.prepare_shadow_draw(pending, buffers, *candidate, frame, assets));
                }
            }

            for cascade_index in 0..shadow_info.cascade_count.min(self.shadows.shadow_camera_buffers.len()) {
                let shadow_camera_uniform = CameraUniform {
                    view_proj: shadow_info.view_proj[cascade_index].to_cols_array_2d(),
                    camera_position: [0.0, 0.0, 0.0, 1.0],
                    camera_forward: [0.0, 0.0, -1.0, 0.0],
                    inverse_view_proj: shadow_info.view_proj[cascade_index].inverse().to_cols_array_2d(),
                };
                self.core.queue.write_buffer(&self.shadows.shadow_camera_buffers[cascade_index], 0, bytemuck::bytes_of(&shadow_camera_uniform));
            }

            for cascade_index in 0..shadow_info.cascade_count.min(self.shadows.shadow_target.layer_views.len()) {
                #[cfg(feature = "profiler")]
                let shadow_timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.shadow_cascades"));
                #[cfg(feature = "profiler")]
                let shadow_timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(shadow_timestamp_indices));
                #[cfg(not(feature = "profiler"))]
                let shadow_timestamp_writes = None;
                let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vetrace directional shadow cascade pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.shadows.shadow_target.layer_views[cascade_index],
                        depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: shadow_timestamp_writes,
                    occlusion_query_set: None,
                });
                shadow_pass.set_pipeline(&self.shadows.shadow_pipeline);
                shadow_pass.set_bind_group(1, &self.shadows.shadow_camera_bind_groups[cascade_index], &[]);
                for draw in &shadow_draws {
                    if !shadow_draw_visible_in_cascade(draw, shadow_info.view_proj[cascade_index]) {
                        continue;
                    }
                    shadow_pass.set_bind_group(0, &draw.material_bind_group, &[]);
                    shadow_pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                    if let Some(index_buffer) = &draw.index_buffer {
                        shadow_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        shadow_pass.draw_indexed(0..draw.index_count, 0, 0..1);
                    } else {
                        shadow_pass.draw(0..draw.vertex_count, 0..1);
                    }
                }
            }
            #[cfg(feature = "profiler")]
            let evsm_started = Instant::now();
            self.run_evsm_passes(encoder, shadow_info);
            #[cfg(feature = "profiler")]
            vetrace_profiler::record_timing("wgpu.game.evsm_passes_cpu_encode", evsm_started.elapsed());
            self.evict_old_shadow_cache_entries(shadow_frame);
        } else {
            // If shadows are disabled, avoid holding stale per-object shadow GPU
            // buffers forever.  The next enabled shadow frame will rebuild only
            // what it needs.
        }
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.shadow_passes_cpu_encode", started.elapsed());
    }
}
