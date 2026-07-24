use super::*;

// Split-out implementation details for `wgpu_window.rs`.

impl WgpuRenderer {
    pub(super) fn run_evsm_passes(&mut self, encoder: &mut wgpu::CommandEncoder, shadow_info: &ShadowInfo) {
        if !shadow_info.enabled || shadow_info.filter_mode != ShadowFilterMode::EvsmBlurred {
            return;
        }

        let Some(evsm_a) = self.shadows.shadow_target.evsm_moments_a.as_ref() else { return; };
        let Some(evsm_b) = self.shadows.shadow_target.evsm_moments_b.as_ref() else { return; };
        let layers = shadow_info
            .cascade_count
            .min(self.shadows.shadow_target.layer_views.len())
            .min(evsm_a.layer_views.len())
            .min(evsm_b.layer_views.len());
        if layers == 0 {
            return;
        }

        let radius = shadow_info.evsm_blur_radius.max(shadow_info.soft_radius).clamp(0.0, 8.0);
        let first_evsm_layer = if layers > 1 { 1 } else { 0 };
        for layer in first_evsm_layer..layers {
            let uniform = Self::evsm_pass_uniform([0.0, 0.0], 0.0, layer, shadow_info.evsm_exponent, self.shadows.shadow_target.size);
            let uniform_buffer = self.evsm_uniform_buffer(uniform);
            let bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vetrace EVSM depth-to-moments bind group"),
                layout: &self.shadows.evsm_moment_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.shadows.shadow_target.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: uniform_buffer.as_entire_binding() },
                ],
            });
            #[cfg(feature = "profiler")]
            let timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.evsm_depth_to_moments"));
            #[cfg(feature = "profiler")]
            let timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(timestamp_indices));
            #[cfg(not(feature = "profiler"))]
            let timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace EVSM depth-to-moments pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &evsm_a.layer_views[layer],
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadows.evsm_moment_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        for layer in first_evsm_layer..layers {
            let uniform = Self::evsm_pass_uniform([1.0, 0.0], radius, layer, shadow_info.evsm_exponent, self.shadows.shadow_target.size);
            let uniform_buffer = self.evsm_uniform_buffer(uniform);
            let bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vetrace EVSM horizontal blur bind group"),
                layout: &self.shadows.evsm_blur_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&evsm_a.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: uniform_buffer.as_entire_binding() },
                ],
            });
            #[cfg(feature = "profiler")]
            let timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.evsm_horizontal_blur"));
            #[cfg(feature = "profiler")]
            let timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(timestamp_indices));
            #[cfg(not(feature = "profiler"))]
            let timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace EVSM horizontal blur pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &evsm_b.layer_views[layer],
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadows.evsm_blur_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        for layer in first_evsm_layer..layers {
            let uniform = Self::evsm_pass_uniform([0.0, 1.0], radius, layer, shadow_info.evsm_exponent, self.shadows.shadow_target.size);
            let uniform_buffer = self.evsm_uniform_buffer(uniform);
            let bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("vetrace EVSM vertical blur bind group"),
                layout: &self.shadows.evsm_blur_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&evsm_b.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: uniform_buffer.as_entire_binding() },
                ],
            });
            #[cfg(feature = "profiler")]
            let timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.evsm_vertical_blur"));
            #[cfg(feature = "profiler")]
            let timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(timestamp_indices));
            #[cfg(not(feature = "profiler"))]
            let timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace EVSM vertical blur pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &evsm_a.layer_views[layer],
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.shadows.evsm_blur_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
