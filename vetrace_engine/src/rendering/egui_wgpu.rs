use egui::{ClippedPrimitive, TexturesDelta};
use egui_wgpu::{Renderer, ScreenDescriptor};
use wgpu::TextureView;

pub struct EguiRenderer {
    renderer: Renderer,
    screen_desc: ScreenDescriptor,
}

impl EguiRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, scale: f32, size: (u32, u32)) -> Self {
        let renderer = Renderer::new(device, format, None, 1);
        let screen_desc = ScreenDescriptor { size_in_pixels: [size.0, size.1], pixels_per_point: scale };
        Self { renderer, screen_desc }
    }

    pub fn set_pixels_per_point(&mut self, scale: f32) {
        self.screen_desc.pixels_per_point = scale;
    }

    pub fn update_screen_rect(&mut self, size: (u32, u32)) {
        self.screen_desc.size_in_pixels = [size.0, size.1];
    }

    pub fn paint_jobs(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &TextureView,
        textures_delta: &TexturesDelta,
        primitives: &[ClippedPrimitive],
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }
        for id in &textures_delta.free {
            self.renderer.free_texture(id);
        }
        self.renderer
            .update_buffers(device, queue, encoder, primitives, &self.screen_desc);
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.renderer.render(&mut rpass, primitives, &self.screen_desc);
        }
    }
}
