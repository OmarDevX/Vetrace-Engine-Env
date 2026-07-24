use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn create_outline_mask_pipeline(
    device: &wgpu::Device,
    surface: GpuSurfaceConfig,
    material_layout: &wgpu::BindGroupLayout,
    camera_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    // The outline stencil pass only needs the old compact fragment interface.
    // Do not use the full GLTF/PBR vertex output here; WGPU validates that
    // every vertex output location is consumed by the fragment entry point.
    let vertex = create_legacy_vertex_module(device);
    let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace outline stencil mask fragment"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(OUTLINE_MASK_FRAGMENT_WGSL)),
    });
    let label = "vetrace outline stencil mask pipeline";
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} layout")),
        bind_group_layouts: &[material_layout, camera_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vertex,
            entry_point: "vs_main",
            buffers: &[GpuVertex::layout()],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Replace,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Replace,
                },
                read_mask: 0xff,
                write_mask: 0xff,
            },
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fragment,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface.format,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            })],
            compilation_options: Default::default(),
        }),
        multiview: None,
    })
}

pub(super) fn create_outline_overlay_pipeline(
    device: &wgpu::Device,
    surface: GpuSurfaceConfig,
    material_layout: &wgpu::BindGroupLayout,
    camera_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    // Same reason as the stencil mask pass: outlines should not force the
    // optional GLTF/PBR uv/color fragment ABI.
    let vertex = create_legacy_vertex_module(device);
    let fragment = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace through-depth outline fragment"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(OUTLINE_FRAGMENT_WGSL)),
    });
    let label = "vetrace through-depth outline pipeline";
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} layout")),
        bind_group_layouts: &[material_layout, camera_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &vertex,
            entry_point: "vs_main",
            buffers: &[GpuVertex::layout()],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                read_mask: 0xff,
                write_mask: 0x00,
            },
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fragment,
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
