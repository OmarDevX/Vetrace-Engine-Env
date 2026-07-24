use super::*;

// Scene and outline render passes for the WGPU frame path.

#[derive(Clone, Copy)]
pub(super) enum SceneOutputTarget<'a> {
    Surface(&'a wgpu::TextureView),
    SsaoSceneColor,
    PostProcessTargetA,
    RenderTexture {
        color: &'a wgpu::TextureView,
        depth: &'a wgpu::TextureView,
        camera: &'a wgpu::BindGroup,
    },
}

impl WgpuRenderer {
    pub(super) fn render_scene_draws(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output: SceneOutputTarget<'_>,
        clear: [f32; 4],
        opaque_draws: &[PreparedDraw],
        transparent_draws: &mut Vec<PreparedDraw>,
        overlay_draws: &mut Vec<PreparedDraw>,
    ) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        #[cfg(feature = "profiler")]
        let scene_timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.scene_pass"));
        #[cfg(feature = "profiler")]
        let scene_timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(scene_timestamp_indices));
        let Some(scene_output_view) = self.scene_output_view(output) else {
            return;
        };
        let scene_depth_view = self.scene_depth_view(output);
        let scene_camera_bind_group = self.scene_camera_bind_group(output);
        {
            #[cfg(not(feature = "profiler"))]
            let scene_timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace wgpu scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: clear[0] as f64, g: clear[1] as f64, b: clear[2] as f64, a: clear[3] as f64 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: scene_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: scene_timestamp_writes,
                occlusion_query_set: None,
            });

            for draw in opaque_draws {
                self.draw_prepared_scene_draw(&mut pass, draw, scene_camera_bind_group);
            }

            if self.environment.environment_sky_enabled {
                pass.set_pipeline(&self.pipelines.sky_pipeline);
                pass.set_bind_group(0, scene_camera_bind_group, &[]);
                pass.set_bind_group(1, &self.environment.environment_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }

            transparent_draws.sort_by(|a, b| b.sort_depth.partial_cmp(&a.sort_depth).unwrap_or(std::cmp::Ordering::Equal));
            for draw in transparent_draws {
                self.draw_prepared_scene_draw(&mut pass, draw, scene_camera_bind_group);
            }

            overlay_draws.sort_by(|a, b| b.sort_depth.partial_cmp(&a.sort_depth).unwrap_or(std::cmp::Ordering::Equal));
            for draw in overlay_draws {
                self.draw_prepared_scene_draw(&mut pass, draw, scene_camera_bind_group);
            }
        }
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.scene_pass_cpu_encode", started.elapsed());
    }

    pub(super) fn render_outline_draws(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output: SceneOutputTarget<'_>,
        outline_draws: &[PreparedOutlineDraw],
    ) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        for outlined in outline_draws {
            #[cfg(feature = "profiler")]
            let outline_timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.outline_passes"));
            #[cfg(feature = "profiler")]
            let outline_timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(outline_timestamp_indices));
            let Some(scene_output_view) = self.scene_output_view(output) else {
                return;
            };
            let scene_depth_view = self.scene_depth_view(output);
            let scene_camera_bind_group = self.scene_camera_bind_group(output);
            #[cfg(not(feature = "profiler"))]
            let outline_timestamp_writes = None;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace wgpu through-depth outline pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_output_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: scene_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0), store: wgpu::StoreOp::Store }),
                }),
                timestamp_writes: outline_timestamp_writes,
                occlusion_query_set: None,
            });

            pass.set_stencil_reference(1);

            pass.set_pipeline(&self.pipelines.outline_mask_pipeline);
            self.draw_outline_part(&mut pass, &outlined.mask, scene_camera_bind_group);

            pass.set_pipeline(&self.pipelines.outline_overlay_pipeline);
            self.draw_outline_part(&mut pass, &outlined.outline, scene_camera_bind_group);
        }
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.outline_pass_cpu_encode", started.elapsed());
    }

    pub(super) fn draw_prepared_scene_draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: &'a PreparedDraw,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.bind_scene_draw_pipeline(pass, draw);
        pass.set_bind_group(0, &draw.material_bind_group, &[]);
        pass.set_bind_group(1, camera_bind_group, &[]);
        pass.set_bind_group(2, &self.environment.environment_bind_group, &[]);
        pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
        if let Some(index_buffer) = &draw.index_buffer {
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        } else {
            pass.draw(0..draw.vertex_count, 0..1);
        }
    }

    pub(super) fn draw_outline_part<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: &'a PreparedDraw,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        pass.set_bind_group(0, &draw.material_bind_group, &[]);
        pass.set_bind_group(1, camera_bind_group, &[]);
        pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
        if let Some(index_buffer) = &draw.index_buffer {
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        } else {
            pass.draw(0..draw.vertex_count, 0..1);
        }
    }

    pub(super) fn scene_output_view<'a>(
        &'a self,
        output: SceneOutputTarget<'a>,
    ) -> Option<&'a wgpu::TextureView> {
        match output {
            SceneOutputTarget::Surface(view) => Some(view),
            SceneOutputTarget::SsaoSceneColor => self
                .post_process
                .ao_target
                .as_ref()
                .map(|target| &target.scene_color.view),
            SceneOutputTarget::PostProcessTargetA => self
                .post_process
                .post_process_target_a
                .as_ref()
                .map(|target| &target.view),
            SceneOutputTarget::RenderTexture { color, .. } => Some(color),
        }
    }

    pub(super) fn scene_depth_view<'a>(&'a self, output: SceneOutputTarget<'a>) -> &'a wgpu::TextureView {
        match output {
            SceneOutputTarget::RenderTexture { depth, .. } => depth,
            _ => &self.core.depth.view,
        }
    }

    pub(super) fn scene_camera_bind_group<'a>(&'a self, output: SceneOutputTarget<'a>) -> &'a wgpu::BindGroup {
        match output {
            SceneOutputTarget::RenderTexture { camera, .. } => camera,
            _ => &self.scene.camera_bind_group,
        }
    }

}
