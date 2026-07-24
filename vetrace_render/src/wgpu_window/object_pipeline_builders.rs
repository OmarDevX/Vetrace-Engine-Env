use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn create_vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace object vertex shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(OBJECT_VERTEX_WGSL)),
    })
}


pub(super) fn create_textured_vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace textured object vertex shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TEXTURED_OBJECT_VERTEX_WGSL)),
    })
}

pub(super) fn create_legacy_vertex_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace legacy object vertex shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(LEGACY_OBJECT_VERTEX_WGSL)),
    })
}

pub(super) fn create_object_pipeline(
    device: &wgpu::Device,
    surface: GpuSurfaceConfig,
    material_layout: &wgpu::BindGroupLayout,
    camera_layout: &wgpu::BindGroupLayout,
    environment_layout: &wgpu::BindGroupLayout,
    fragment_wgsl: &str,
    label: &str,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    let vertex = create_vertex_module(device);
    let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{label} fragment")),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(fragment_wgsl)),
    });
    create_object_pipeline_from_modules(device, surface, material_layout, camera_layout, environment_layout, &vertex, &fragment, label, depth_write, depth_compare, cull_mode)
}

pub(super) fn create_object_pipeline_from_modules(
    device: &wgpu::Device,
    surface: GpuSurfaceConfig,
    material_layout: &wgpu::BindGroupLayout,
    camera_layout: &wgpu::BindGroupLayout,
    environment_layout: &wgpu::BindGroupLayout,
    vertex: &wgpu::ShaderModule,
    fragment: &wgpu::ShaderModule,
    label: &str,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} layout")),
        bind_group_layouts: &[material_layout, camera_layout, environment_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: vertex,
            entry_point: "vs_main",
            buffers: &[GpuVertex::layout()],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: depth_write,
            depth_compare,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: fragment,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview: None,
    })
}
