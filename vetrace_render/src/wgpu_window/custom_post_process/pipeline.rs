use super::*;

pub(super) fn custom_post_process_uniform_for_pass(
    pass: &CustomPostProcessPass,
    frame: &RenderFrame,
    pass_index: usize,
    width: u32,
    height: u32,
    previous_view_proj: Mat4,
    history_valid: bool,
) -> CustomPostProcessUniform {
    let mut params = [[0.0_f32; 4]; 8];
    for (index, value) in pass.params.iter().take(32).enumerate() {
        params[index / 4][index % 4] = *value;
    }
    let input_mode = match pass.input {
        PostProcessInput::SceneColor => 0.0,
        PostProcessInput::SceneColorDepth => 1.0,
    };
    let camera = camera_uniform_for(&frame.camera, width, height);
    CustomPostProcessUniform {
        params,
        screen_time: [width.max(1) as f32, height.max(1) as f32, frame.settings.time_seconds, pass_index as f32],
        info: [pass.params.len().min(32) as f32, input_mode, if history_valid { 1.0 } else { 0.0 }, 0.0],
        view_proj: camera.view_proj,
        inverse_view_proj: camera.inverse_view_proj,
        camera_position: camera.camera_position,
        camera_forward: camera.camera_forward,
        previous_view_proj: previous_view_proj.to_cols_array_2d(),
    }
}

pub(super) fn create_custom_post_process_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("vetrace custom post-process layout"),
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
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
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
        ],
    })
}

pub(super) fn create_custom_post_process_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    wgsl: &str,
    label: &str,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(wgsl.to_string())),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} layout")),
        bind_group_layouts: &[bind_group_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview: None,
    })
}

pub(super) fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) const FXAA_WGSL: &str = include_str!("../fxaa.wgsl");

pub(super) const DEFAULT_CUSTOM_POST_PROCESS_WGSL: &str = r#"
struct CustomPostProcessUniform {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    p4: vec4<f32>,
    p5: vec4<f32>,
    p6: vec4<f32>,
    p7: vec4<f32>,
    screen_time: vec4<f32>, // width, height, time_seconds, pass_index
    info: vec4<f32>,        // param_count, input_mode, history_valid, reserved
};

@group(0) @binding(0)
var scene_color: texture_2d<f32>;

@group(0) @binding(1)
var scene_sampler: sampler;

@group(0) @binding(2)
var scene_depth: texture_depth_2d;

@group(0) @binding(3)
var<uniform> post: CustomPostProcessUniform;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    var p = vec2<f32>(-1.0, -3.0);
    if (vertex_index == 1u) { p = vec2<f32>(3.0, 1.0); }
    if (vertex_index == 2u) { p = vec2<f32>(-1.0, 1.0); }
    return p;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let p = fullscreen_triangle_position(vertex_index);
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = p * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    return textureSample(scene_color, scene_sampler, input.uv);
}
"#;
