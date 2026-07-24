use super::*;

// Split-out implementation details for `wgpu_window.rs`.

#[derive(Clone, Copy, Debug)]
pub(super) enum MaterialTextureFallback {
    White,
    Normal,
}

pub(super) fn upload_texture_asset(device: &wgpu::Device, queue: &wgpu::Queue, label: &str, texture: &TextureAsset, srgb: bool) -> GpuTextureResource {
    if srgb {
        GpuTextureResource::from_rgba8_srgb(device, queue, label, texture.width, texture.height, &texture.rgba8)
    } else {
        GpuTextureResource::from_rgba8_linear(device, queue, label, texture.width, texture.height, &texture.rgba8)
    }
}

pub(super) fn material_texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

pub(super) fn create_shadow_material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace alpha-tested shadow material layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(super) fn create_evsm_moment_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace EVSM depth-to-moments layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

pub(super) fn create_evsm_blur_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace EVSM separable blur layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}


impl WgpuRenderer {
    pub(super) fn sync_baked_lightmap_atlas(&mut self, atlas: Option<&BakedLightmapAtlas>) {
        let Some(atlas) = atlas else {
            self.scene.baked_lightmap_texture = None;
            return;
        };
        if self
            .scene
            .baked_lightmap_texture
            .as_ref()
            .is_some_and(|(id, texture)| {
                *id == atlas.id && texture.width == atlas.width && texture.height == atlas.height
            })
        {
            return;
        }
        let texture = GpuTextureResource::from_rgba16_float(
            &self.core.device,
            &self.core.queue,
            "vetrace baked-lightmap RGBA16F atlas",
            atlas.width,
            atlas.height,
            atlas.rgba16f.as_slice(),
        );
        self.scene.baked_lightmap_texture = Some((atlas.id, texture));
    }

    pub(super) fn baked_lightmap_view(&self, atlas_id: Option<u64>) -> &wgpu::TextureView {
        match (atlas_id, self.scene.baked_lightmap_texture.as_ref()) {
            (Some(expected), Some((actual, texture))) if expected == *actual => &texture.view,
            _ => &self.scene.white_linear_texture.view,
        }
    }
}
