use super::*;

// Bind-group layouts and camera buffers used during WGPU initialization.

pub(super) struct InitialLayouts {
    pub(super) material_layout: wgpu::BindGroupLayout,
    pub(super) camera_layout: wgpu::BindGroupLayout,
    pub(super) environment_layout: wgpu::BindGroupLayout,
    pub(super) camera_buffer: wgpu::Buffer,
    pub(super) camera_bind_group: wgpu::BindGroup,
    pub(super) shadow_camera_buffers: Vec<wgpu::Buffer>,
    pub(super) shadow_camera_bind_groups: Vec<wgpu::BindGroup>,
    pub(super) shadow_material_layout: wgpu::BindGroupLayout,
    pub(super) evsm_moment_layout: wgpu::BindGroupLayout,
    pub(super) evsm_blur_layout: wgpu::BindGroupLayout,
    pub(super) ssao_layout: wgpu::BindGroupLayout,
    pub(super) ssao_blur_layout: wgpu::BindGroupLayout,
    pub(super) ssao_composite_layout: wgpu::BindGroupLayout,
    pub(super) custom_post_process_layout: wgpu::BindGroupLayout,
}

pub(super) fn create_initial_layouts(device: &wgpu::Device) -> InitialLayouts {
    let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace material/custom params layout"),
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
            material_texture_layout_entry(1), // base color / albedo, sampled as sRGB
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            material_texture_layout_entry(3), // tangent-space normal, linear data
            material_texture_layout_entry(4), // glTF metallic-roughness, linear data
            material_texture_layout_entry(5), // occlusion, linear data
            material_texture_layout_entry(6), // emissive, sampled as sRGB
            wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            material_texture_layout_entry(10), // baked RGBA16F lightmap atlas, linear data
            material_texture_layout_entry(11), // custom render texture slot 0
            material_texture_layout_entry(12), // custom render texture slot 1
            material_texture_layout_entry(13), // custom render texture slot 2
            material_texture_layout_entry(14), // custom render texture slot 3
            wgpu::BindGroupLayoutEntry {
                binding: 15,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            }, // clamp-to-edge render-texture sampler
        ],
    });
    let environment_layout = create_environment_layout(device);
    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace camera layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vetrace camera uniform"),
        size: std::mem::size_of::<CameraUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("vetrace camera bind group"),
        layout: &camera_layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
    });
    let mut shadow_camera_buffers = Vec::with_capacity(SHADOW_CASCADE_COUNT);
    let mut shadow_camera_bind_groups = Vec::with_capacity(SHADOW_CASCADE_COUNT);
    for cascade in 0..SHADOW_CASCADE_COUNT {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("vetrace shadow cascade camera uniform {cascade}")),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("vetrace shadow cascade camera bind group {cascade}")),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        shadow_camera_buffers.push(buffer);
        shadow_camera_bind_groups.push(bind_group);
    }

    InitialLayouts {
        material_layout,
        camera_layout,
        environment_layout,
        camera_buffer,
        camera_bind_group,
        shadow_camera_buffers,
        shadow_camera_bind_groups,
        shadow_material_layout: create_shadow_material_layout(device),
        evsm_moment_layout: create_evsm_moment_layout(device),
        evsm_blur_layout: create_evsm_blur_layout(device),
        ssao_layout: create_ssao_layout(device),
        ssao_blur_layout: create_ssao_blur_layout(device),
        ssao_composite_layout: create_ssao_composite_layout(device),
        custom_post_process_layout: create_custom_post_process_layout(device),
    }
}
