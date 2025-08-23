use vetrace_engine::app::{app, App};
use vetrace_engine::components::components::{
    Atmosphere, Bloom, CameraAttachment, DirectionalLight, FreeFlightControls, PostProcessing,
    Transform,
};
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;
#[cfg(feature = "wgpu")]
use wgpu::util::DeviceExt;

const GLITCH_SHADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>;
    @location(0) uv: vec2<f32>;
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0),
    );
    var out: VsOut;
    let pos = positions[idx];
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

@group(0) @binding(0) var<uniform> u_time: f32;

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let n = fract(sin((v.uv.y + u_time) * 200.0) * 43758.5453);
    if n > 0.95 {
        let c = vec3<f32>(n, 1.0 - n, n * 0.5);
        return vec4<f32>(c, 0.4);
    }
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
"#;

struct EmissiveGlow {
    time: f32,
    #[cfg(feature = "wgpu")]
    glitch_pipeline: Option<std::sync::Arc<wgpu::RenderPipeline>>,
    #[cfg(feature = "wgpu")]
    glitch_bind_group: Option<std::sync::Arc<wgpu::BindGroup>>,
    #[cfg(feature = "wgpu")]
    time_buffer: Option<wgpu::Buffer>,
}

impl App for EmissiveGlow {
    fn setup(&mut self, engine: &mut Engine) {
        // Spawn a planet so the atmosphere has something to wrap around
        let mut planet = Object::default();
        planet.is_cube = false;
        planet.radius = 100.0;
        planet.position = [0.0, -planet.radius, 0.0];
        planet.color = [0.0, 64.0, 12.0];
        if let Some(mut actor) = engine.spawn_object_as_actor(planet) {
            actor.with_bundle(Atmosphere::default());
        }

        // A second planet with its own atmosphere to demonstrate multiple instances
        let mut planet2 = Object::default();
        planet2.is_cube = false;
        planet2.radius = 50.0;
        planet2.position = [200.0, -planet2.radius, 0.0];
        planet2.color = [64.0, 16.0, 64.0];
        if let Some(mut actor) = engine.spawn_object_as_actor(planet2) {
            let mut atmo = Atmosphere::default();
            atmo.planet_radius = 50.0;
            atmo.atmo_radius = 60.0;
            actor.with_bundle(atmo);
        }

        // Spawn a cube with a high emission value above the surface
        let mut cube = Object::default();
        cube.position = [0.0, 1.0, 0.0];
        cube.color = [255.0, 140.0, 60.0];
        cube.emission = 0.0;
        engine.spawn_object(cube);

        // Create a camera looking at the cube
        let cam = engine.spawn_empty("camera");
        engine.world.insert(
            cam,
            Transform {
                position: [0.0, 0.0, -5.0],
                ..Default::default()
            },
        );

        // Enable bloom so the emission appears as a glow
        let bloom = Bloom {
            threshold: 1.0,
            intensity: 2.0,
            spread: 4.0,
            iterations: 7,
            ..Default::default()
        };
        engine.world.insert(
            cam,
            PostProcessing {
                bloom: Some(bloom),
                ..Default::default()
            },
        );

        // Add a basic directional light and controls
        engine.world.insert(
            cam,
            DirectionalLight {
                direction: [-1.0, -1.0, -1.0],
                color: [255.0, 255.0, 255.0],
                intensity: 1.0,
            },
        );
        engine.world.insert(cam, CameraAttachment::default());
        engine.world.insert(cam, FreeFlightControls::default());

        // Build glitch shader pipeline and register UI callback
        #[cfg(feature = "wgpu")]
        {
            let device = engine.renderer.device();
            let format = engine.renderer.surface_format();

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("glitch shader"),
                source: wgpu::ShaderSource::Wgsl(GLITCH_SHADER.into()),
            });

            let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glitch time"),
                contents: bytemuck::cast_slice(&[0.0f32]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("glitch bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bind_group = std::sync::Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("glitch bg"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: time_buffer.as_entire_binding(),
                }],
            }));

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("glitch layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = std::sync::Arc::new(device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("glitch pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            }));

            self.glitch_pipeline = Some(pipeline.clone());
            self.glitch_bind_group = Some(bind_group.clone());
            self.time_buffer = Some(time_buffer);

            struct GlitchPainter {
                pipeline: std::sync::Arc<wgpu::RenderPipeline>,
                bind_group: std::sync::Arc<wgpu::BindGroup>,
            }

            impl egui_wgpu::CallbackTrait for GlitchPainter {
                fn paint<'a>(
                    &'a self,
                    _info: egui::PaintCallbackInfo,
                    rpass: &mut wgpu::RenderPass<'a>,
                    _resources: &'a egui_wgpu::CallbackResources,
                ) {
                    rpass.set_pipeline(&self.pipeline);
                    rpass.set_bind_group(0, &self.bind_group, &[]);
                    rpass.draw(0..3, 0..1);
                }
            }

            engine.add_ui_callback(move |ctx, _engine| {
                use egui::{Id, LayerId, Order, Shape};
                let rect = ctx.screen_rect();
                let cb = egui_wgpu::Callback::new_paint_callback(
                    rect,
                    GlitchPainter {
                        pipeline: pipeline.clone(),
                        bind_group: bind_group.clone(),
                    },
                );
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("glitch")))
                    .add(Shape::Callback(cb));
                Ok(())
            });
        }
    }

    fn update(&mut self, engine: &mut Engine, delta: f32) {
        self.time += delta;
        #[cfg(feature = "wgpu")]
        if let Some(buf) = &self.time_buffer {
            engine
                .renderer
                .queue()
                .write_buffer(buf, 0, bytemuck::cast_slice(&[self.time]));
        }
    }

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app().with_title("Emissive Glow").run(EmissiveGlow {
        time: 0.0,
        #[cfg(feature = "wgpu")]
        glitch_pipeline: None,
        #[cfg(feature = "wgpu")]
        glitch_bind_group: None,
        #[cfg(feature = "wgpu")]
        time_buffer: None,
    })
}
