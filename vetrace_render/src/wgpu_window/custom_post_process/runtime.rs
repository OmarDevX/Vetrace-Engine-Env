use super::*;

// Split-out implementation details for `wgpu_window.rs`.

impl WgpuRenderer {
    pub(super) fn fxaa_enabled_for_frame(frame: &RenderFrame) -> bool {
        frame.settings.anti_aliasing_mode == AntiAliasingMode::Fxaa
    }

    pub(super) fn ssr_enabled_for_frame(frame: &RenderFrame) -> bool {
        frame.custom_post_process_passes.iter().any(|pass| {
            pass.enabled && pass.pass_id == crate::resources::SCREEN_SPACE_REFLECTIONS_PASS_ID
        })
    }

    pub(super) fn ssr_needs_final_resolve(frame: &RenderFrame) -> bool {
        !Self::fxaa_enabled_for_frame(frame)
            && frame
                .custom_post_process_passes
                .iter()
                .filter(|pass| pass.enabled)
                .last()
                .is_some_and(|pass| pass.pass_id == crate::resources::SCREEN_SPACE_REFLECTIONS_PASS_ID)
    }

    pub(super) fn post_process_pass_count(frame: &RenderFrame) -> usize {
        frame.custom_post_process_passes.iter().filter(|pass| pass.enabled).count()
            + if Self::fxaa_enabled_for_frame(frame) { 1 } else { 0 }
            + if Self::ssr_needs_final_resolve(frame) { 1 } else { 0 }
    }

    pub(super) fn ensure_post_process_targets_size(
        &mut self,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
        needs_ping_pong: bool,
    ) {
        let recreate = |target: &Option<GpuTextureResource>| {
            target
                .as_ref()
                .map(|target| target.width != width.max(1) || target.height != height.max(1) || target.format != surface_format)
                .unwrap_or(true)
        };
        if recreate(&self.post_process.post_process_target_a) {
            self.post_process.post_process_target_a = Some(GpuTextureResource::new_render_target(
                &self.core.device,
                "vetrace post-process target A",
                width,
                height,
                surface_format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            ));
        }
        if needs_ping_pong {
            if recreate(&self.post_process.post_process_target_b) {
                self.post_process.post_process_target_b = Some(GpuTextureResource::new_render_target(
                    &self.core.device,
                    "vetrace post-process target B",
                    width,
                    height,
                    surface_format,
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                ));
            }
        } else {
            self.post_process.post_process_target_b = None;
        }
    }

    pub(super) fn ensure_ssr_history_size(
        &mut self,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
    ) {
        let recreate = self.post_process.ssr_history.as_ref().map_or(true, |target| {
            target.width != width.max(1)
                || target.height != height.max(1)
                || target.format != surface_format
        });
        if recreate {
            self.post_process.ssr_history = Some(GpuTextureResource::new_render_target(
                &self.core.device,
                "vetrace SSR temporal history",
                width,
                height,
                surface_format,
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            ));
            self.post_process.ssr_history_valid = false;
        }
    }

    pub(super) fn release_ssr_history_if_unused(&mut self) {
        self.post_process.ssr_history = None;
        self.post_process.ssr_history_valid = false;
        self.post_process.previous_post_process_view_proj = Mat4::IDENTITY;
    }

    pub(super) fn release_post_process_targets_if_unused(&mut self) {
        self.post_process.post_process_target_a = None;
        self.post_process.post_process_target_b = None;
    }

    pub(super) fn run_post_process_chain(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        final_output_view: &wgpu::TextureView,
    ) {
        let custom_passes: Vec<&CustomPostProcessPass> = frame
            .custom_post_process_passes
            .iter()
            .filter(|pass| pass.enabled)
            .collect();
        let fxaa_enabled = Self::fxaa_enabled_for_frame(frame);
        let needs_ssr_resolve = Self::ssr_needs_final_resolve(frame);
        let total_passes = custom_passes.len()
            + if fxaa_enabled { 1 } else { 0 }
            + if needs_ssr_resolve { 1 } else { 0 };
        if total_passes == 0
            || self.post_process.post_process_target_a.is_none()
            || (total_passes > 1 && self.post_process.post_process_target_b.is_none())
        {
            return;
        }

        self.ensure_custom_post_process_uniform_buffers(custom_passes.len());
        let history_valid_at_frame_start = self.post_process.ssr_history_valid;
        let mut wrote_ssr_history = false;
        let mut input_is_a = true;
        for (pass_index, pass) in custom_passes.iter().enumerate() {
            let pipeline_key = self.ensure_custom_post_process_pipeline(pass, assets);
            let Some(pipeline) = self
                .post_process
                .custom_post_process_pipelines
                .get(&pipeline_key)
            else {
                eprintln!("WGPU: custom post-process pipeline `{pipeline_key}` was not cached");
                return;
            };

            let uniform = custom_post_process_uniform_for_pass(
                pass,
                frame,
                pass_index,
                self.core.config.width,
                self.core.config.height,
                self.post_process.previous_post_process_view_proj,
                history_valid_at_frame_start,
            );
            let uniform_buffer = &self.post_process.custom_post_process_uniform_buffers[pass_index];
            self.core.queue.write_buffer(uniform_buffer, 0, bytemuck::bytes_of(&uniform));

            let last = pass_index + 1 == total_passes;
            let Some(input_view) = self.post_process_input_view(input_is_a) else {
                return;
            };
            let output_is_a = !input_is_a;
            let target_view = if last {
                final_output_view
            } else {
                let Some(target_view) = self.post_process_ping_pong_target_view(input_is_a) else {
                    return;
                };
                target_view
            };
            let history_view = self
                .post_process
                .ssr_history
                .as_ref()
                .filter(|_| history_valid_at_frame_start)
                .map(|history| &history.view)
                .unwrap_or(&self.scene.black_linear_texture.view);
            let bind_group = self.create_post_process_bind_group(
                &format!("vetrace custom post-process bind group: {}", pass.pass_id),
                input_view,
                history_view,
                uniform_buffer,
            );
            self.encode_post_process_pass(
                encoder,
                &format!("vetrace custom post-process pass: {}", pass.pass_id),
                pipeline,
                &bind_group,
                target_view,
                "wgpu.gpu.custom_post_process",
            );

            if pass.pass_id == crate::resources::SCREEN_SPACE_REFLECTIONS_PASS_ID && !last {
                if let (Some(source), Some(history)) = (
                    self.post_process_resource(output_is_a),
                    self.post_process.ssr_history.as_ref(),
                ) {
                    encoder.copy_texture_to_texture(
                        wgpu::ImageCopyTexture {
                            texture: &source._texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::ImageCopyTexture {
                            texture: &history._texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width: self.core.config.width.max(1),
                            height: self.core.config.height.max(1),
                            depth_or_array_layers: 1,
                        },
                    );
                    wrote_ssr_history = true;
                }
            }

            if !last {
                input_is_a = output_is_a;
            }
        }

        if fxaa_enabled {
            let Some(input_view) = self.post_process_input_view(input_is_a) else {
                return;
            };
            let history_view = self
                .post_process
                .ssr_history
                .as_ref()
                .filter(|_| history_valid_at_frame_start)
                .map(|history| &history.view)
                .unwrap_or(&self.scene.black_linear_texture.view);
            let bind_group = self.create_post_process_bind_group(
                "vetrace FXAA bind group",
                input_view,
                history_view,
                &self.post_process.custom_post_process_uniform_buffer,
            );
            self.encode_post_process_pass(
                encoder,
                "vetrace FXAA pass",
                &self.post_process.fxaa_pipeline,
                &bind_group,
                final_output_view,
                "wgpu.gpu.fxaa",
            );
        } else if needs_ssr_resolve {
            let Some(input_view) = self.post_process_input_view(input_is_a) else {
                return;
            };
            let bind_group = self.create_post_process_bind_group(
                "vetrace SSR final resolve bind group",
                input_view,
                &self.scene.black_linear_texture.view,
                &self.post_process.custom_post_process_uniform_buffer,
            );
            self.encode_post_process_pass(
                encoder,
                "vetrace SSR final resolve",
                &self.post_process.post_process_copy_pipeline,
                &bind_group,
                final_output_view,
                "wgpu.gpu.ssr_resolve",
            );
        }

        if wrote_ssr_history {
            self.post_process.ssr_history_valid = true;
        }
        self.post_process.previous_post_process_view_proj = camera_matrix_for_surface(
            frame,
            self.core.config.width,
            self.core.config.height,
        );
    }

    pub(super) fn post_process_input_view(
        &self,
        input_is_a: bool,
    ) -> Option<&wgpu::TextureView> {
        self.post_process_resource(input_is_a)
            .map(|target| &target.view)
    }

    pub(super) fn post_process_ping_pong_target_view(
        &self,
        input_is_a: bool,
    ) -> Option<&wgpu::TextureView> {
        self.post_process_resource(!input_is_a)
            .map(|target| &target.view)
    }

    pub(super) fn post_process_resource(&self, is_a: bool) -> Option<&GpuTextureResource> {
        if is_a { self.post_process.post_process_target_a.as_ref() } else { self.post_process.post_process_target_b.as_ref() }
    }

    pub(super) fn ensure_custom_post_process_uniform_buffers(&mut self, count: usize) {
        while self.post_process.custom_post_process_uniform_buffers.len() < count {
            let index = self.post_process.custom_post_process_uniform_buffers.len();
            self.post_process.custom_post_process_uniform_buffers.push(self.core.device.create_buffer(
                &wgpu::BufferDescriptor {
                    label: Some(&format!("vetrace custom post-process uniform slot {index}")),
                    size: std::mem::size_of::<CustomPostProcessUniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                },
            ));
        }
    }

    pub(super) fn create_post_process_bind_group(
        &self,
        label: &str,
        input_view: &wgpu::TextureView,
        history_view: &wgpu::TextureView,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.post_process.custom_post_process_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.scene.screen_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.core.depth.sample_view) },
                wgpu::BindGroupEntry { binding: 3, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(history_view) },
            ],
        })
    }

    pub(super) fn encode_post_process_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &str,
        pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        target_view: &wgpu::TextureView,
        _profiler_label: &'static str,
    ) {
        // Keep this helper borrow-only so pipelines and ping-pong texture views can
        // be borrowed from `self` without cloning handles. CPU timings still cover
        // the complete post chain; a dedicated timestamp slot can be added later
        // if per-pass GPU profiling becomes necessary.
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub(super) fn ensure_custom_post_process_pipeline(&mut self, pass: &CustomPostProcessPass, assets: Option<&RenderAssets>) -> String {
        let source = resolve_text_source(pass.wgsl_source.as_deref(), pass.asset_path.as_deref(), assets)
            .unwrap_or_else(|| DEFAULT_CUSTOM_POST_PROCESS_WGSL.to_string());
        let key = format!(
            "{}|fmt={:?}|shader={:016x}",
            pass.pass_id,
            self.core.surface_view_format,
            stable_hash(source.as_bytes())
        );
        if !self.post_process.custom_post_process_pipelines.contains_key(&key) {
            let pipeline = create_custom_post_process_pipeline(
                &self.core.device,
                &self.post_process.custom_post_process_layout,
                &source,
                &format!("vetrace custom post-process pipeline: {}", pass.pass_id),
                self.core.surface_view_format,
            );
            self.post_process.custom_post_process_pipelines.insert(key.clone(), pipeline);
        }
        key
    }
}
