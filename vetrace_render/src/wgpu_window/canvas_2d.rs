use std::borrow::Cow;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};

use super::*;
use crate::backend::RenderSprite2D;
use crate::components::{BlendMode2D, TextureFilter2D, TextureHandle};

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Camera2DUniformGpu {
    /// xy = camera center, z = cos(rotation), w = sin(rotation)
    center_rotation: [f32; 4],
    /// xy = complete surface size, z = pixels/world-unit, w = camera pixel snap
    surface_scale_snap: [f32; 4],
    /// xy = viewport origin, zw = viewport size in physical pixels
    viewport_rect: [f32; 4],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct Sprite2DInstanceGpu {
    /// xy = world X axis, zw = world origin (bottom-left after pivot)
    axis_x_origin: [f32; 4],
    /// xy = world Y axis, zw = UV at the bottom-left vertex
    axis_y_uv_origin: [f32; 4],
    /// xy = UV delta over the quad, z = alpha cutoff, w = sprite pixel snap
    uv_delta_cutoff_snap: [f32; 4],
    tint: [f32; 4],
}

impl Sprite2DInstanceGpu {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4
    ];
}

struct PreparedCanvas2DBatch {
    instances: Range<u32>,
    texture: Option<TextureHandle>,
    filter: TextureFilter2D,
    blend_mode: BlendMode2D,
}

pub(super) struct Canvas2DRendererState {
    texture_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    nearest_sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    alpha_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    multiply_pipeline: wgpu::RenderPipeline,
}

impl Canvas2DRendererState {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vetrace 2D camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vetrace 2D texture layout"),
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
            ],
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vetrace 2D camera uniform"),
            size: std::mem::size_of::<Camera2DUniformGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace 2D camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let instance_capacity = 1;
        let instance_buffer = create_instance_buffer(device, instance_capacity);
        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vetrace 2D nearest sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vetrace 2D linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let alpha_pipeline = create_sprite_pipeline(
            device,
            format,
            &camera_layout,
            &texture_layout,
            "vetrace 2D alpha pipeline",
            alpha_blend_state(),
        );
        let additive_pipeline = create_sprite_pipeline(
            device,
            format,
            &camera_layout,
            &texture_layout,
            "vetrace 2D additive pipeline",
            additive_blend_state(),
        );
        let multiply_pipeline = create_sprite_pipeline(
            device,
            format,
            &camera_layout,
            &texture_layout,
            "vetrace 2D multiply pipeline",
            multiply_blend_state(),
        );
        Self {
            texture_layout,
            camera_buffer,
            camera_bind_group,
            instance_buffer,
            instance_capacity,
            nearest_sampler,
            linear_sampler,
            alpha_pipeline,
            additive_pipeline,
            multiply_pipeline,
        }
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.instance_capacity {
            return;
        }
        self.instance_capacity = required.next_power_of_two().max(1);
        self.instance_buffer = create_instance_buffer(device, self.instance_capacity);
    }

    fn pipeline(&self, blend_mode: BlendMode2D) -> &wgpu::RenderPipeline {
        match blend_mode {
            BlendMode2D::Alpha => &self.alpha_pipeline,
            BlendMode2D::Additive => &self.additive_pipeline,
            BlendMode2D::Multiply => &self.multiply_pipeline,
        }
    }

    fn sampler(&self, filter: TextureFilter2D) -> &wgpu::Sampler {
        match filter {
            TextureFilter2D::Nearest => &self.nearest_sampler,
            TextureFilter2D::Linear => &self.linear_sampler,
        }
    }
}

impl WgpuRenderer {
    pub(super) fn render_canvas_2d(
        &mut self,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        if frame.sprites_2d.is_empty() {
            return;
        }

        let surface_size = Vec2::new(
            self.core.config.width.max(1) as f32,
            self.core.config.height.max(1) as f32,
        );
        let mut instances = Vec::with_capacity(frame.sprites_2d.len());
        let mut batches: Vec<PreparedCanvas2DBatch> = Vec::new();
        for sprite in &frame.sprites_2d {
            if !sprite_intersects_camera(sprite, &frame.camera_2d, surface_size) {
                continue;
            }
            self.ensure_texture(sprite.sprite.texture, assets, true);
            let texture_size = sprite
                .sprite
                .texture
                .and_then(|handle| assets.and_then(|assets| assets.textures.get(&handle.0)))
                .map(|texture| Vec2::new(texture.width.max(1) as f32, texture.height.max(1) as f32))
                .unwrap_or(Vec2::ONE);
            instances.push(sprite_instance(sprite, texture_size));
            let start = (instances.len() - 1) as u32;
            let end = instances.len() as u32;
            if let Some(batch) = batches.last_mut() {
                if batch.texture == sprite.sprite.texture
                    && batch.filter == sprite.sprite.filter
                    && batch.blend_mode == sprite.canvas.blend_mode
                {
                    batch.instances.end = end;
                    continue;
                }
            }
            batches.push(PreparedCanvas2DBatch {
                instances: start..end,
                texture: sprite.sprite.texture,
                filter: sprite.sprite.filter,
                blend_mode: sprite.canvas.blend_mode,
            });
        }

        #[cfg(feature = "profiler")]
        {
            vetrace_profiler::record_counter("render.2d.visible", instances.len() as f64, "sprites");
            vetrace_profiler::record_counter(
                "render.2d.culled",
                frame.sprites_2d.len().saturating_sub(instances.len()) as f64,
                "sprites",
            );
            vetrace_profiler::record_counter("render.2d.batches", batches.len() as f64, "batches");
        }
        if instances.is_empty() {
            return;
        }

        self.canvas_2d
            .ensure_instance_capacity(&self.core.device, instances.len());
        self.core.queue.write_buffer(
            &self.canvas_2d.instance_buffer,
            0,
            bytemuck::cast_slice(&instances),
        );
        let rotation = frame.camera_2d.rotation;
        let (viewport_origin, viewport_size) = frame.camera_2d.viewport_rect_px(surface_size);
        let camera = Camera2DUniformGpu {
            center_rotation: [
                frame.camera_2d.position.x,
                frame.camera_2d.position.y,
                rotation.cos(),
                rotation.sin(),
            ],
            surface_scale_snap: [
                surface_size.x,
                surface_size.y,
                frame.camera_2d.pixels_per_world_unit(),
                if frame.camera_2d.pixel_snap { 1.0 } else { 0.0 },
            ],
            viewport_rect: [
                viewport_origin.x,
                viewport_origin.y,
                viewport_size.x,
                viewport_size.y,
            ],
        };
        self.core.queue.write_buffer(
            &self.canvas_2d.camera_buffer,
            0,
            bytemuck::bytes_of(&camera),
        );

        let mut texture_bind_groups = Vec::with_capacity(batches.len());
        for batch in &batches {
            let texture_view = self.sprite_texture_view(batch.texture, true);
            texture_bind_groups.push(self.core.device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    label: Some("vetrace 2D texture bind group"),
                    layout: &self.canvas_2d.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(
                                self.canvas_2d.sampler(batch.filter),
                            ),
                        },
                    ],
                },
            ));
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vetrace 2D canvas pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_scissor_rect(
            viewport_origin.x.floor().max(0.0) as u32,
            viewport_origin.y.floor().max(0.0) as u32,
            viewport_size.x.ceil().max(1.0) as u32,
            viewport_size.y.ceil().max(1.0) as u32,
        );
        pass.set_bind_group(0, &self.canvas_2d.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.canvas_2d.instance_buffer.slice(..));
        for (batch, bind_group) in batches.iter().zip(texture_bind_groups.iter()) {
            pass.set_pipeline(self.canvas_2d.pipeline(batch.blend_mode));
            pass.set_bind_group(1, bind_group, &[]);
            pass.draw(0..6, batch.instances.clone());
        }
    }
}

fn sprite_intersects_camera(
    sprite: &RenderSprite2D,
    camera: &crate::resources::Camera2D,
    surface_size: Vec2,
) -> bool {
    let size = sprite.sprite.size.abs();
    let local_min = -size * sprite.sprite.pivot;
    let local_max = local_min + size;
    let corners = [
        Vec2::new(local_min.x, local_min.y),
        Vec2::new(local_max.x, local_min.y),
        Vec2::new(local_max.x, local_max.y),
        Vec2::new(local_min.x, local_max.y),
    ];
    let camera_rotation = glam::Mat2::from_angle(-camera.rotation);
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for corner in corners {
        let world = sprite.transform.translation
            + sprite.transform.rotation * Vec3::new(
                corner.x * sprite.transform.scale.x,
                corner.y * sprite.transform.scale.y,
                0.0,
            );
        let local = camera_rotation * (world.truncate() - camera.position);
        min = min.min(local);
        max = max.max(local);
    }
    let half = camera.visible_half_extents(surface_size);
    max.x >= -half.x && min.x <= half.x && max.y >= -half.y && min.y <= half.y
}

fn sprite_instance(sprite: &RenderSprite2D, texture_size: Vec2) -> Sprite2DInstanceGpu {
    let size = sprite.sprite.size.abs().max(Vec2::splat(0.0001));
    let axis_x = (sprite.transform.rotation * Vec3::X).truncate()
        * sprite.transform.scale.x
        * size.x;
    let axis_y = (sprite.transform.rotation * Vec3::Y).truncate()
        * sprite.transform.scale.y
        * size.y;
    let pivot = sprite.sprite.pivot;
    let origin = sprite.transform.translation.truncate() - axis_x * pivot.x - axis_y * pivot.y;

    let source = sprite.sprite.source_rect_px.unwrap_or(crate::components::Rect2D {
        min: Vec2::ZERO,
        size: texture_size,
    });
    let min = source.min.max(Vec2::ZERO).min(texture_size);
    let max = (source.min + source.size)
        .max(Vec2::ZERO)
        .min(texture_size);
    let mut left = min.x / texture_size.x.max(1.0);
    let mut right = max.x / texture_size.x.max(1.0);
    let mut top = min.y / texture_size.y.max(1.0);
    let mut bottom = max.y / texture_size.y.max(1.0);
    if sprite.sprite.flip_x {
        std::mem::swap(&mut left, &mut right);
    }
    if sprite.sprite.flip_y {
        std::mem::swap(&mut top, &mut bottom);
    }

    Sprite2DInstanceGpu {
        axis_x_origin: [axis_x.x, axis_x.y, origin.x, origin.y],
        axis_y_uv_origin: [axis_y.x, axis_y.y, left, bottom],
        uv_delta_cutoff_snap: [
            right - left,
            top - bottom,
            sprite.sprite.alpha_cutoff.clamp(0.0, 1.0),
            if sprite.sprite.pixel_snap { 1.0 } else { 0.0 },
        ],
        tint: sprite.sprite.tint.to_array(),
    }
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace 2D sprite instance buffer"),
        size: (capacity.max(1) * std::mem::size_of::<Sprite2DInstanceGpu>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_sprite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    camera_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    label: &str,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace 2D sprite shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
            "canvas_2d/sprite_2d.wgsl"
        ))),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("vetrace 2D sprite pipeline layout"),
        bind_group_layouts: &[camera_layout, texture_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Sprite2DInstanceGpu>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &Sprite2DInstanceGpu::ATTRIBUTES,
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn alpha_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

fn additive_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

fn multiply_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::Dst,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_layouts_match_wgsl_vec4_packing() {
        assert_eq!(std::mem::size_of::<Camera2DUniformGpu>(), 48);
        assert_eq!(std::mem::align_of::<Camera2DUniformGpu>(), 16);
        assert_eq!(std::mem::size_of::<Sprite2DInstanceGpu>(), 64);
        assert_eq!(std::mem::align_of::<Sprite2DInstanceGpu>(), 16);
    }

    #[test]
    fn sprite_shader_semantically_validates_as_wgsl() {
        let module = naga::front::wgsl::parse_str(include_str!("canvas_2d/sprite_2d.wgsl"))
            .expect("2D sprite WGSL must parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("2D sprite WGSL must pass Naga semantic validation");
    }
}
