use super::*;

impl WgpuRenderer {
    pub(super) fn render_reflection_capture_face(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        scene_frame: u64,
        probe: &RenderReflectionProbe,
        target: &GpuReflectionCaptureTarget,
        face: u32,
    ) {
        let camera = reflection_capture_camera(probe, face);
        let camera_uniform = camera_uniform_for(&camera, target.resolution, target.resolution);
        self.core.queue
            .write_buffer(&target.camera_buffers[face as usize], 0, bytemuck::bytes_of(&camera_uniform));

        let pending_draws = self.prepare_pending_draws_for_reflection_capture(
            frame,
            assets,
            camera.position,
            probe,
        );
        let mut capture_frame = frame.clone();
        capture_frame.camera = camera.clone();
        let capture_shadow_info = if probe.capture_shadows {
            let shadow_light = primary_shadow_light(&capture_frame);
            let candidates = build_directional_shadow_candidates(
                &pending_draws,
                shadow_light.is_some(),
                capture_frame.settings.shadow_max_vertices as usize,
                camera.position,
                capture_frame.settings.shadow_max_distance,
            );
            let shadow_info = shadow_info_for_frame(&capture_frame, shadow_light, &candidates);
            self.render_reflection_capture_shadow_passes(
                encoder,
                &capture_frame,
                assets,
                &pending_draws,
                &shadow_info,
                &candidates,
                target,
                face,
            );
            shadow_info
        } else {
            disabled_shadow_info(&capture_frame.settings)
        };
        let PreparedSceneDraws { opaque_draws, mut transparent_draws, .. } = self.prepare_scene_draws_for_frame(
            &capture_frame,
            assets,
            pending_draws,
            &capture_shadow_info,
            scene_frame,
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vetrace reflection capture face"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.face_views[face as usize],
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // Draw the sky first at far depth, then let captured scene geometry
        // overwrite it. This is robust regardless of tiny far-plane/depth
        // precision differences and guarantees the baked map contains objects.
        if self.environment.capture_sky_enabled {
            pass.set_pipeline(&self.pipelines.capture_sky_pipeline);
            pass.set_bind_group(0, &target.camera_bind_groups[face as usize], &[]);
            pass.set_bind_group(1, &self.environment.capture_environment_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        for draw in &opaque_draws {
            self.draw_reflection_capture_object(
                &mut pass,
                draw,
                &target.camera_bind_groups[face as usize],
                false,
            );
        }
        if probe.capture_transparent {
            transparent_draws.sort_by(|a, b| {
                b.sort_depth
                    .partial_cmp(&a.sort_depth)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for draw in &transparent_draws {
                self.draw_reflection_capture_object(
                    &mut pass,
                    draw,
                    &target.camera_bind_groups[face as usize],
                    true,
                );
            }
        }
    }

    pub(super) fn draw_reflection_capture_object<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        draw: &'a PreparedDraw,
        camera_bind_group: &'a wgpu::BindGroup,
        transparent: bool,
    ) {
        let pipeline = match &draw.pipeline {
            PipelineKind::Default => {
                if transparent { &self.pipelines.capture_transparent_pipeline } else { &self.pipelines.capture_default_pipeline }
            }
            PipelineKind::DefaultDoubleSided => {
                if transparent { &self.pipelines.capture_transparent_double_sided_pipeline } else { &self.pipelines.capture_default_double_sided_pipeline }
            }
            PipelineKind::Transparent => &self.pipelines.capture_transparent_pipeline,
            PipelineKind::TransparentDoubleSided => &self.pipelines.capture_transparent_double_sided_pipeline,
            PipelineKind::Custom { key, .. } => {
                let Some(pipeline) = self.pipelines.custom_capture_pipelines.get(key) else { return; };
                pipeline
            }
            PipelineKind::OutlineMask | PipelineKind::OutlineOverlay => return,
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &draw.material_bind_group, &[]);
        pass.set_bind_group(1, camera_bind_group, &[]);
        pass.set_bind_group(2, &self.environment.capture_environment_bind_group, &[]);
        pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
        if let Some(index_buffer) = &draw.index_buffer {
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        } else {
            pass.draw(0..draw.vertex_count, 0..1);
        }
    }
}
