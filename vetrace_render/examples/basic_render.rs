use glam::{Quat, Vec3};
use vetrace_core::{App, AppBuilder, Engine, Transform};
use vetrace_render::{Material, PrimitiveShape, RenderBundle, RenderPlugin, RenderSettings, Renderable, Shape};

struct DemoApp;

impl App for DemoApp {
    fn setup(&mut self, engine: &mut Engine) {
        engine
            .spawn_actor("demo_cube")
            .with(Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .bundle(RenderBundle {
                shape: Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE },
                material: Material {
                    base_color: Vec3::new(0.2, 0.7, 1.0),
                    ..Default::default()
                },
                renderable: Renderable::default(),
            })
            .build();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Render Demo".to_string(),
            width: 960,
            height: 540,
            ..Default::default()
        })
        .add_plugin(RenderPlugin::new())
        .run_frames(DemoApp, 120, 1.0 / 60.0)
}
