use super::*;

impl WgpuRenderer {
    pub(super) fn render_reflection_capture_shadow_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        pending_draws: &[PendingDraw<'_>],
        shadow_info: &ShadowInfo,
        shadow_candidates: &[ShadowCandidate],
        target: &GpuReflectionCaptureTarget,
        face: u32,
    ) {
        if !shadow_info.enabled || shadow_candidates.is_empty() {
            return;
        }
        self.shadows.shadow_cache_frame = self.shadows.shadow_cache_frame.wrapping_add(1);
        let shadow_frame = self.shadows.shadow_cache_frame;
        let mut shadow_draws = Vec::with_capacity(shadow_candidates.len());
        for candidate in shadow_candidates {
            let pending = &pending_draws[candidate.index];
            let buffers = self.geometry_buffers_for(
                pending.geometry_key,
                pending.geometry_signature,
                &pending.geometry,
                shadow_frame,
            );
            if buffers.draw_count() > 0 {
                shadow_draws.push(self.prepare_shadow_draw(
                    pending,
                    buffers,
                    *candidate,
                    frame,
                    assets,
                ));
            }
        }

        let face_base = face as usize * SHADOW_CASCADE_COUNT;
        let cascade_count = shadow_info
            .cascade_count
            .min(SHADOW_CASCADE_COUNT)
            .min(self.shadows.shadow_target.layer_views.len());
        for cascade in 0..cascade_count {
            let index = face_base + cascade;
            let camera_uniform = CameraUniform {
                view_proj: shadow_info.view_proj[cascade].to_cols_array_2d(),
                camera_position: [0.0, 0.0, 0.0, 1.0],
                camera_forward: [0.0, 0.0, -1.0, 0.0],
                inverse_view_proj: shadow_info.view_proj[cascade].inverse().to_cols_array_2d(),
            };
            self.core.queue.write_buffer(
                &target.shadow_camera_buffers[index],
                0,
                bytemuck::bytes_of(&camera_uniform),
            );
        }

        for cascade in 0..cascade_count {
            let index = face_base + cascade;
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace reflection capture directional shadow cascade"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadows.shadow_target.layer_views[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            shadow_pass.set_pipeline(&self.shadows.shadow_pipeline);
            shadow_pass.set_bind_group(1, &target.shadow_camera_bind_groups[index], &[]);
            for draw in &shadow_draws {
                if !shadow_draw_visible_in_cascade(draw, shadow_info.view_proj[cascade]) {
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
        self.run_evsm_passes(encoder, shadow_info);
        self.evict_old_shadow_cache_entries(shadow_frame);
    }
}
