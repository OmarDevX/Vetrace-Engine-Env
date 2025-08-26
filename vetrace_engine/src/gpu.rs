use std::sync::Arc;

use anyhow::Result;
use wgpu::util::DeviceExt;

#[derive(Clone, Debug)]
pub struct TextureHandle(pub Arc<GpuTexture>);

#[derive(Debug)]
pub struct GpuTexture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub format: wgpu::TextureFormat,
    pub size: wgpu::Extent3d,
    pub is_srgb: bool,
}

impl GpuTexture {
    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
        srgb: bool,
        label: &str,
    ) -> Result<Self> {
        let format = if srgb {
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8Unorm
        };
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });

        Ok(Self { view, sampler, format, size, is_srgb: srgb })
    }
}

#[derive(Clone, Debug)]
pub struct MeshHandle(pub Arc<GpuMesh>);

#[derive(Debug)]
pub struct GpuMesh {
    pub name: String,
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
    pub morph_targets: Option<GpuMorphTargets>,
}

#[derive(Debug)]
pub struct GpuMorphTargets {
    pub position_buffers: Vec<wgpu::Buffer>,
    pub normal_buffers: Vec<wgpu::Buffer>,
    pub target_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub tan: [f32; 4],
    pub uv: [f32; 2],
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

impl GpuMesh {
    pub fn from_cpu(
        device: &wgpu::Device,
        name: &str,
        verts: &[Vertex],
        indices: &[u32],
    ) -> Result<Self> {
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("vbuf:{name}")),
            contents: bytemuck::cast_slice(verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("ibuf:{name}")),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Ok(Self {
            name: name.into(),
            vbuf,
            ibuf,
            index_count: indices.len() as u32,
            morph_targets: None,
        })
    }

    pub fn from_cpu_with_morph_targets(
        device: &wgpu::Device,
        name: &str,
        verts: &[Vertex],
        indices: &[u32],
        morph_targets: Option<&crate::assets::MorphTargetSet>,
    ) -> Result<Self> {
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("vbuf:{name}")),
            contents: bytemuck::cast_slice(verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ibuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("ibuf:{name}")),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let gpu_morph_targets = if let Some(morph_set) = morph_targets {
            let mut position_buffers = Vec::new();
            let mut normal_buffers = Vec::new();

            for target in &morph_set.targets {
                // Create position buffer for this morph target
                let pos_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("morph_pos:{name}:{}", target.name)),
                    contents: bytemuck::cast_slice(&target.vertex_positions),
                    usage: wgpu::BufferUsages::STORAGE,
                });
                position_buffers.push(pos_buf);

                // Create normal buffer if available
                if let Some(ref normals) = target.vertex_normals {
                    let norm_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("morph_norm:{name}:{}", target.name)),
                        contents: bytemuck::cast_slice(normals),
                        usage: wgpu::BufferUsages::STORAGE,
                    });
                    normal_buffers.push(norm_buf);
                } else {
                    // Create a dummy buffer with zero normals
                    let zero_normals = vec![[0.0f32; 3]; target.vertex_positions.len()];
                    let norm_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("morph_norm_zero:{name}:{}", target.name)),
                        contents: bytemuck::cast_slice(&zero_normals),
                        usage: wgpu::BufferUsages::STORAGE,
                    });
                    normal_buffers.push(norm_buf);
                }
            }

            Some(GpuMorphTargets {
                position_buffers,
                normal_buffers,
                target_count: morph_set.targets.len(),
            })
        } else {
            None
        };

        Ok(Self {
            name: name.into(),
            vbuf,
            ibuf,
            index_count: indices.len() as u32,
            morph_targets: gpu_morph_targets,
        })
    }
}
