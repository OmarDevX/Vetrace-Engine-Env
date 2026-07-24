use super::*;

use std::hash::{Hash, Hasher};

pub(super) fn create_environment_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace environment layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::CubeArray,
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(super) fn create_initial_environment_resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> InitialEnvironmentResources {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vetrace environment cubemap pool"),
        size: wgpu::Extent3d {
            width: ENVIRONMENT_CUBEMAP_FACE_SIZE,
            height: ENVIRONMENT_CUBEMAP_FACE_SIZE,
            depth_or_array_layers: ENVIRONMENT_CUBEMAP_CAPACITY * 6,
        },
        mip_level_count: ENVIRONMENT_CUBEMAP_MIP_COUNT,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: ENVIRONMENT_TEXTURE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("vetrace environment cube-array view"),
        format: Some(ENVIRONMENT_TEXTURE_FORMAT),
        dimension: Some(wgpu::TextureViewDimension::CubeArray),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(ENVIRONMENT_CUBEMAP_MIP_COUNT),
        base_array_layer: 0,
        array_layer_count: Some(ENVIRONMENT_CUBEMAP_CAPACITY * 6),
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("vetrace environment cubemap sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: (ENVIRONMENT_CUBEMAP_MIP_COUNT - 1) as f32,
        ..Default::default()
    });
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace environment uniform"),
        size: std::mem::size_of::<EnvironmentUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let capture_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace reflection capture environment uniform"),
        size: std::mem::size_of::<EnvironmentUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let probe_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace reflection probe buffer"),
        size: (std::mem::size_of::<GpuReflectionProbe>() * MAX_REFLECTION_PROBES) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let brdf_lut = create_environment_brdf_lut(device, queue);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("vetrace environment bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: probe_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: uniform_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&brdf_lut.view) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&brdf_lut.sampler) },
        ],
    });
    let capture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("vetrace reflection capture environment bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: probe_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: capture_uniform_buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&brdf_lut.view) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&brdf_lut.sampler) },
        ],
    });
    let prefilter_layout = create_reflection_prefilter_layout(device);
    let prefilter_pipeline = create_reflection_prefilter_pipeline(device, &prefilter_layout);

    InitialEnvironmentResources {
        cubemap_pool: GpuEnvironmentCubemapPool {
            texture,
            slots: HashMap::new(),
            signature: 0,
        },
        uniform_buffer,
        probe_buffer,
        bind_group,
        capture_uniform_buffer,
        capture_bind_group,
        prefilter_layout,
        prefilter_pipeline,
        brdf_lut,
    }
}
