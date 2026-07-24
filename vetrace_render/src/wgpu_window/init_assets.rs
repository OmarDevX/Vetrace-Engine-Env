use super::*;

// Default textures, samplers, and post-process uniform buffers.

pub(super) struct InitialGpuAssets {
    pub(super) texture_sampler: wgpu::Sampler,
    pub(super) shadow_sampler: wgpu::Sampler,
    pub(super) screen_sampler: wgpu::Sampler,
    pub(super) white_srgb_texture: GpuTextureResource,
    pub(super) white_linear_texture: GpuTextureResource,
    pub(super) black_linear_texture: GpuTextureResource,
    pub(super) neutral_normal_texture: GpuTextureResource,
    pub(super) dummy_evsm_moments: EvsmMomentTarget,
    pub(super) ssao_uniform_buffer: wgpu::Buffer,
    pub(super) custom_post_process_uniform_buffer: wgpu::Buffer,
}

pub(super) fn create_initial_gpu_assets(device: &wgpu::Device, queue: &wgpu::Queue) -> InitialGpuAssets {
    let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("vetrace material texture sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    let white_srgb_texture = GpuTextureResource::from_rgba8_srgb(
        device,
        queue,
        "vetrace white srgb material texture",
        1,
        1,
        &[255, 255, 255, 255],
    );
    let white_linear_texture = GpuTextureResource::from_rgba8_linear(
        device,
        queue,
        "vetrace white linear material texture",
        1,
        1,
        &[255, 255, 255, 255],
    );
    let black_linear_texture = GpuTextureResource::from_rgba8_linear(
        device,
        queue,
        "vetrace black linear fallback texture",
        1,
        1,
        &[0, 0, 0, 255],
    );
    let neutral_normal_texture = GpuTextureResource::from_rgba8_linear(
        device,
        queue,
        "vetrace neutral normal texture",
        1,
        1,
        &[128, 128, 255, 255],
    );
    let dummy_evsm_moments = EvsmMomentTarget::new(device, "vetrace dummy EVSM moments", 1, 1);
    let screen_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("vetrace screen-space postprocess sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let ssao_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace SSAO uniform"),
        size: std::mem::size_of::<SsaoUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let custom_post_process_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace custom post-process uniform"),
        size: std::mem::size_of::<CustomPostProcessUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("vetrace directional shadow comparison sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    });

    InitialGpuAssets {
        texture_sampler,
        shadow_sampler,
        screen_sampler,
        white_srgb_texture,
        white_linear_texture,
        black_linear_texture,
        neutral_normal_texture,
        dummy_evsm_moments,
        ssao_uniform_buffer,
        custom_post_process_uniform_buffer,
    }
}
