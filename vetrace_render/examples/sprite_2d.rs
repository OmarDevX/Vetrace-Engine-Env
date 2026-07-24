//! Minimal feature-gated 2D canvas example.
//!
//! Run from the workspace root:
//! `cargo run -p vetrace_render --example sprite_2d --features wgpu_window,render_2d`

use glam::{Quat, Vec2, Vec3, Vec4};
use vetrace_core::{App, AppBuilder, Engine, Entity, InputState, Transform};
use vetrace_render::{
    Camera2D, CanvasItem2D, Render2dPlugin, RenderPlugin, RenderSettings, Sprite2D,
    TextureFilter2D,
};

struct Sprite2DExample {
    sprite: Entity,
    time: f32,
}

impl App for Sprite2DExample {
    fn setup(&mut self, engine: &mut Engine) {
        engine.insert_resource(Camera2D {
            pixels_per_unit: 96.0,
            pixel_snap: true,
            ..Camera2D::default()
        });
        self.sprite = engine
            .spawn_actor("Feature-gated Sprite 2D")
            .with(Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .with(Sprite2D {
                size: Vec2::new(3.0, 2.0),
                tint: Vec4::new(0.18, 0.72, 1.0, 0.92),
                filter: TextureFilter2D::Nearest,
                pixel_snap: true,
                ..Sprite2D::default()
            })
            .with(CanvasItem2D::default())
            .build()
            .entity();
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.time += dt;
        if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(self.sprite) {
            transform.rotation = Quat::from_rotation_z(self.time * 0.45);
        }
        if engine
            .get_resource::<InputState>()
            .map(InputState::quit_requested)
            .unwrap_or(false)
        {
            engine.stop();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Sprite 2D".to_owned(),
            width: 960,
            height: 540,
            cursor_grab: false,
            cursor_visible: true,
            clear_color: [0.015, 0.02, 0.04, 1.0],
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .add_plugin(Render2dPlugin::new())
        .run_until_stopped(
            Sprite2DExample { sprite: Entity::INVALID, time: 0.0 },
            None,
            1.0 / 60.0,
        )
}
