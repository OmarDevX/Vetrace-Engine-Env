use super::*;

impl WgpuRenderer {
    pub(super) fn prefilter_reflection_capture_mip(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &GpuReflectionCaptureTarget,
        slot: u32,
        mip_level: u32,
        configured_sample_count: u32,
    ) {
        let roughness = mip_level as f32 / (ENVIRONMENT_CUBEMAP_MIP_COUNT - 1).max(1) as f32;
        let sample_count = if mip_level == 0 { 1 } else { configured_sample_count.clamp(16, 256) };
        for face in 0..6_u32 {
            let uniform = ReflectionPrefilterUniform {
                face_sample_count: [face, sample_count, mip_level, 0],
                params: [roughness, 0.0, 0.0, 0.0],
            };
            self.core.queue.write_buffer(
                &target.prefilter_uniform_buffers[face as usize],
                0,
                bytemuck::bytes_of(&uniform),
            );
            let destination = self.environment.environment_cubemap_pool.texture.create_view(
                &wgpu::TextureViewDescriptor {
                    label: Some("vetrace filtered reflection cubemap face"),
                    format: Some(ENVIRONMENT_TEXTURE_FORMAT),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip_level,
                    mip_level_count: Some(1),
                    base_array_layer: slot * 6 + face,
                    array_layer_count: Some(1),
                },
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace reflection GGX prefilter pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &destination,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.environment.reflection_prefilter_pipeline);
            pass.set_bind_group(0, &target.prefilter_bind_groups[face as usize], &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
