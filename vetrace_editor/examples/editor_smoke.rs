use glam::{Quat, Vec3};
use vetrace_core::{App, AppBuilder, Engine, Transform};
use vetrace_editor::{spawn_editor_overlay_marker, spawn_editor_test_cube, EditorPlugin};
use vetrace_render::{Atmosphere, Camera, DirectionalLight, Material, PrimitiveShape, RenderPlugin, RenderSettings, Renderable, Shape};

struct EditorSmokeDemo;

impl App for EditorSmokeDemo {
    fn setup(&mut self, engine: &mut Engine) {
        engine.insert_resource(Camera {
            position: Vec3::new(3.0, 2.5, 6.0),
            target: Vec3::new(0.0, 0.6, 0.0),
            ..Camera::default()
        });

        let ground = engine.spawn_actor("Ground").build().entity();
        engine.raw_world_mut().insert(ground, Transform {
            translation: Vec3::new(0.0, -0.05, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        engine.raw_world_mut().insert(ground, Shape { primitive: PrimitiveShape::Cube, size: Vec3::new(8.0, 0.1, 8.0) });
        engine.raw_world_mut().insert(ground, Renderable { mesh: None, material: None, visible: true });
        engine.raw_world_mut().insert(ground, Material { base_color: Vec3::new(0.45, 0.45, 0.45), ..Material::default() });

        spawn_editor_test_cube(engine, "Blue Cube", Vec3::new(-1.2, 0.55, 0.0));
        spawn_editor_test_cube(engine, "Editable Cube", Vec3::new(1.2, 0.55, 0.0));
        spawn_editor_overlay_marker(engine);

        let light = engine.spawn_actor("Sun").build().entity();
        engine.raw_world_mut().insert(light, DirectionalLight::default());
        let sky = engine.spawn_actor("Sky").build().entity();
        engine.raw_world_mut().insert(sky, Atmosphere::default());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Editor Smoke Demo".to_string(),
            cursor_grab: false,
            cursor_visible: true,
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .add_plugin(EditorPlugin::new())
        .run_until_stopped(EditorSmokeDemo, None, 1.0 / 60.0)
}
