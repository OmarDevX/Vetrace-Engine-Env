use super::*;

impl GpuReflectionCaptureTarget {
    pub(super) fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        prefilter_layout: &wgpu::BindGroupLayout,
        resolution: u32,
        label: &str,
    ) -> Self {
        let resolution = resolution.clamp(32, 1024).next_power_of_two().min(1024);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("vetrace reflection capture cubemap: {label}")),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: ENVIRONMENT_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let cube_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("vetrace reflection capture cube view: {label}")),
            format: Some(ENVIRONMENT_TEXTURE_FORMAT),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(6),
        });
        let face_views = (0..6)
            .map(|face| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("vetrace reflection capture face {face}: {label}")),
                    format: Some(ENVIRONMENT_TEXTURE_FORMAT),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    base_array_layer: face,
                    array_layer_count: Some(1),
                })
            })
            .collect();
        let depth = DepthTarget::new(device, resolution, resolution);
        // Each face owns a camera buffer. This allows more than one face to be
        // encoded before queue submission without every pass observing the last
        // camera write.
        let mut camera_buffers = Vec::with_capacity(6);
        let mut camera_bind_groups = Vec::with_capacity(6);
        for face in 0..6 {
            let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("vetrace reflection capture camera face {face}: {label}")),
                size: std::mem::size_of::<CameraUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("vetrace reflection capture camera bind group face {face}: {label}")),
                layout: camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
            });
            camera_buffers.push(camera_buffer);
            camera_bind_groups.push(camera_bind_group);
        }
        let mut shadow_camera_buffers = Vec::with_capacity(6 * SHADOW_CASCADE_COUNT);
        let mut shadow_camera_bind_groups = Vec::with_capacity(6 * SHADOW_CASCADE_COUNT);
        for face in 0..6 {
            for cascade in 0..SHADOW_CASCADE_COUNT {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!(
                        "vetrace reflection capture shadow camera face {face} cascade {cascade}: {label}"
                    )),
                    size: std::mem::size_of::<CameraUniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!(
                        "vetrace reflection capture shadow camera bind group face {face} cascade {cascade}: {label}"
                    )),
                    layout: camera_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });
                shadow_camera_buffers.push(buffer);
                shadow_camera_bind_groups.push(bind_group);
            }
        }
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("vetrace reflection capture sampler: {label}")),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mut prefilter_uniform_buffers = Vec::with_capacity(6);
        let mut prefilter_bind_groups = Vec::with_capacity(6);
        for face in 0..6 {
            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("vetrace reflection prefilter uniform face {face}: {label}")),
                size: std::mem::size_of::<ReflectionPrefilterUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("vetrace reflection prefilter bind group face {face}: {label}")),
                layout: prefilter_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&cube_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
            prefilter_uniform_buffers.push(uniform_buffer);
            prefilter_bind_groups.push(bind_group);
        }
        Self {
            _texture: texture,
            _cube_view: cube_view,
            face_views,
            depth,
            camera_buffers,
            camera_bind_groups,
            shadow_camera_buffers,
            shadow_camera_bind_groups,
            prefilter_uniform_buffers,
            prefilter_bind_groups,
            resolution,
        }
    }
}
