use super::*;
use crate::components::Outline;

// Split-out implementation details for `wgpu_backend.rs`.

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct OutlineUniform {
    pub color: [f32; 4],
    pub texel_size_thickness: [f32; 4],
}

impl OutlineUniform {
    pub fn new(outline: &Outline, width: u32, height: u32) -> Self {
        Self {
            color: [outline.color.x, outline.color.y, outline.color.z, 1.0],
            texel_size_thickness: [
                1.0 / width.max(1) as f32,
                1.0 / height.max(1) as f32,
                outline.thickness.max(1.0),
                0.0,
            ],
        }
    }
}

pub struct WgpuOutlinePass {
    pub shader: wgpu::ShaderModule,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub pipeline: wgpu::RenderPipeline,
    pub sampler: wgpu::Sampler,
}

impl WgpuOutlinePass {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vetrace outline pass shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(OUTLINE_PASS_WGSL)),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vetrace outline bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vetrace outline pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vetrace outline pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vetrace outline mask sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..wgpu::SamplerDescriptor::default()
        });
        Self { shader, bind_group_layout, pipeline_layout, pipeline, sampler }
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mask_view: &wgpu::TextureView,
        outline: &Outline,
        width: u32,
        height: u32,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let uniform = OutlineUniform::new(outline, width, height);
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vetrace outline params"),
            size: std::mem::size_of::<OutlineUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace outline bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(mask_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buffer.as_entire_binding() },
            ],
        });
        (uniform_buffer, bind_group)
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, bind_group: &'a wgpu::BindGroup) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
