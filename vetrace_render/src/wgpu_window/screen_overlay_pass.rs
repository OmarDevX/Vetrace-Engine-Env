use super::*;

// Screen-space overlay and egui presentation passes for WgpuRenderTarget.

impl WgpuRenderer {
    pub(super) fn render_screen_overlays_and_egui(
        &mut self,
        frame: &RenderFrame,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        if !frame.overlays.is_empty() {
            let overlay_vertices = super::overlay_geometry::overlay_vertices(&frame.overlays, self.core.config.width as f32, self.core.config.height as f32);
            if !overlay_vertices.is_empty() {
                let vertex_buffer = self.core.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vetrace overlay vertices"),
                    contents: bytemuck::cast_slice(&overlay_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                #[cfg(feature = "profiler")]
                let overlay_timestamp_indices = self.optional.gpu_timestamp_profiler.as_mut().and_then(|profiler| profiler.reserve_pass("wgpu.gpu.overlay_pass"));
                #[cfg(feature = "profiler")]
                let overlay_timestamp_writes = self.optional.gpu_timestamp_profiler.as_ref().and_then(|profiler| profiler.timestamp_writes_for(overlay_timestamp_indices));
                #[cfg(not(feature = "profiler"))]
                let overlay_timestamp_writes = None;
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("vetrace wgpu overlay pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: overlay_timestamp_writes,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&self.pipelines.overlay_pipeline);
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.draw(0..overlay_vertices.len() as u32, 0..1);
            }
        }
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("wgpu.game.overlay_cpu_encode", started.elapsed());

        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        let egui_started = Instant::now();
        #[cfg(feature = "egui_render")]
        self.render_egui_overlay(frame, encoder, view);
        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        vetrace_profiler::record_timing("wgpu.game.egui_cpu_encode", egui_started.elapsed());

    }
}
