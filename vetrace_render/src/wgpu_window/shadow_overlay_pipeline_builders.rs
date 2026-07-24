use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn create_shadow_pipeline(
    device: &wgpu::Device,
    shadow_material_layout: &wgpu::BindGroupLayout,
    camera_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace directional shadow shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADOW_WGSL)),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("vetrace directional shadow pipeline layout"),
        bind_group_layouts: &[shadow_material_layout, camera_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("vetrace directional shadow pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[GpuVertex::layout()],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: SHADOW_DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: Default::default(),
            bias: wgpu::DepthBiasState {
                constant: 1,
                slope_scale: 1.0,
                clamp: 0.0,
            },
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[],
            compilation_options: Default::default(),
        }),
        multiview: None,
    })
}

pub(super) fn create_overlay_pipeline(device: &wgpu::Device, surface: GpuSurfaceConfig) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("vetrace overlay shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(OVERLAY_WGSL)),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("vetrace overlay pipeline layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("vetrace overlay pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[OverlayVertex::layout()],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
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

pub(super) fn is_transparent_pipeline(pipeline: &PipelineKind) -> bool {
    matches!(
        pipeline,
        PipelineKind::Transparent
            | PipelineKind::TransparentDoubleSided
            | PipelineKind::Custom { bucket: CustomShaderRenderBucket::Transparent, .. }
            | PipelineKind::Custom { bucket: CustomShaderRenderBucket::Overlay, .. }
    )
}

pub(super) fn is_overlay_pipeline(pipeline: &PipelineKind) -> bool {
    matches!(pipeline, PipelineKind::Custom { bucket: CustomShaderRenderBucket::Overlay, .. })
}

pub(super) fn material_pipeline_kind(material: &Material) -> PipelineKind {
    let transparent = material.alpha_mode == AlphaMode::Blend
        || (material.alpha_mode == AlphaMode::Opaque && material.alpha < 0.999);
    match (transparent, material.double_sided) {
        (true, true) => PipelineKind::TransparentDoubleSided,
        (true, false) => PipelineKind::Transparent,
        (false, true) => PipelineKind::DefaultDoubleSided,
        (false, false) => PipelineKind::Default,
    }
}

pub(super) fn alpha_mode_code(alpha_mode: AlphaMode) -> f32 {
    match alpha_mode {
        AlphaMode::Opaque => 0.0,
        AlphaMode::Mask => 1.0,
        AlphaMode::Blend => 2.0,
    }
}
