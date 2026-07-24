use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) enum SsaoCompositeOutput<'a> {
    Surface(&'a wgpu::TextureView),
    PostProcessTargetA,
}

impl WgpuRenderer {
    pub(super) fn ao_enabled_for_frame(frame: &RenderFrame) -> bool {
        frame.settings.ambient_occlusion_mode == AmbientOcclusionMode::Ssao
            && frame.settings.ssao_intensity > 0.001
            && frame.settings.ssao_sample_count > 0
    }

    pub(super) fn ensure_ao_target_size(&mut self, width: u32, height: u32, surface_format: wgpu::TextureFormat) {
        let width = width.max(1);
        let height = height.max(1);
        let recreate = self
            .post_process
            .ao_target
            .as_ref()
            .map(|target| target.width != width || target.height != height || target.surface_format != surface_format)
            .unwrap_or(true);
        if recreate {
            self.post_process.ao_target = Some(AmbientOcclusionTarget::new(&self.core.device, width, height, surface_format));
        }
    }

    pub(super) fn release_ao_target_if_unused(&mut self) {
        if self.post_process.ao_target.is_some() {
            self.post_process.ao_target = None;
        }
    }

    pub(super) fn ssao_uniform_for_frame(&self, frame: &RenderFrame) -> SsaoUniform {
        SsaoUniform {
            params0: [
                self.core.config.width.max(1) as f32,
                self.core.config.height.max(1) as f32,
                frame.settings.ssao_radius_pixels.max(1.0),
                frame.settings.ssao_intensity.max(0.0),
            ],
            params1: [
                frame.settings.ssao_bias.max(0.00001),
                frame.settings.ssao_sample_count.clamp(4, 12) as f32,
                frame.camera.near.max(0.0001),
                frame.camera.far.max(frame.camera.near + 0.001),
            ],
            params2: [frame.settings.ssao_blur_radius.max(0.0), 0.0, 0.0, 0.0],
        }
    }

    pub(super) fn run_ssao_and_composite(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        output: SsaoCompositeOutput<'_>,
    ) {
        let Some(target) = self.post_process.ao_target.as_ref() else { return; };
        let output_view = match output {
            SsaoCompositeOutput::Surface(view) => view,
            SsaoCompositeOutput::PostProcessTargetA => {
                let Some(target) = self.post_process.post_process_target_a.as_ref() else { return; };
                &target.view
            }
        };
        let uniform = self.ssao_uniform_for_frame(frame);
        self.core.queue.write_buffer(&self.post_process.ssao_uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let ssao_bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace SSAO bind group"),
            layout: &self.post_process.ssao_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.core.depth.sample_view) },
                wgpu::BindGroupEntry { binding: 1, resource: self.post_process.ssao_uniform_buffer.as_entire_binding() },
            ],
        });
        {
            #[cfg(feature = "profiler")]
            let timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.ssao_raw"));
            #[cfg(feature = "profiler")]
            let timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(timestamp_indices));
            #[cfg(not(feature = "profiler"))]
            let timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace SSAO pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.raw.view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.post_process.ssao_pipeline);
            pass.set_bind_group(0, &ssao_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        let blur_bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace SSAO blur bind group"),
            layout: &self.post_process.ssao_blur_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&target.raw.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.core.depth.sample_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.scene.screen_sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: self.post_process.ssao_uniform_buffer.as_entire_binding() },
            ],
        });
        {
            #[cfg(feature = "profiler")]
            let timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.ssao_blur"));
            #[cfg(feature = "profiler")]
            let timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(timestamp_indices));
            #[cfg(not(feature = "profiler"))]
            let timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace SSAO depth-aware blur pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.blurred.view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.post_process.ssao_blur_pipeline);
            pass.set_bind_group(0, &blur_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        let composite_bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace SSAO composite bind group"),
            layout: &self.post_process.ssao_composite_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&target.scene_color.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&target.blurred.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.scene.screen_sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: self.post_process.ssao_uniform_buffer.as_entire_binding() },
            ],
        });
        {
            #[cfg(feature = "profiler")]
            let timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.ssao_composite"));
            #[cfg(feature = "profiler")]
            let timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(timestamp_indices));
            #[cfg(not(feature = "profiler"))]
            let timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace SSAO composite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.post_process.ssao_composite_pipeline);
            pass.set_bind_group(0, &composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
