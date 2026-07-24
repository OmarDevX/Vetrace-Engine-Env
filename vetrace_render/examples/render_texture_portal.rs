//! Generic render-to-texture camera displayed by a game-side custom shader.
//!
//! Run from the workspace root:
//! `cargo run -p vetrace_render --example render_texture_portal --features wgpu_window`
//!
//! The renderer knows only about a named secondary camera and four generic
//! texture slots. The portal appearance and texture mapping live in WGSL.

use glam::{Quat, Vec3};
use vetrace_core::{App, AppBuilder, Engine, InputState, Transform};
use vetrace_render::{
    AdapterPreference, Camera, CustomShaderMaterial, CustomShaderVertexInterface,
    DirectionalLight, Material,
    PrimitiveShape, RenderBundle, RenderLayers, RenderPlugin, RenderSettings,
    RenderTextureCamera, Renderable, Shape,
};

const PORTAL_SHADER: &str = include_str!("render_texture_portal.wgsl");
const PORTAL_LAYER: u32 = 1 << 8;

struct RenderTexturePortalExample {
    time: f32,
}

impl App for RenderTexturePortalExample {
    fn setup(&mut self, engine: &mut Engine) {
        engine.insert_resource(Camera {
            position: Vec3::new(0.0, 2.4, 7.2),
            target: Vec3::new(0.0, 1.65, 0.0),
            fov_y_radians: 52.0_f32.to_radians(),
            ..Camera::default()
        });

        spawn_cube(
            engine,
            "Ground",
            Vec3::new(0.0, -0.25, -0.5),
            Vec3::new(11.0, 0.5, 11.0),
            Vec3::new(0.08, 0.10, 0.14),
        );
        spawn_cube(
            engine,
            "Orange Tower",
            Vec3::new(-2.4, 1.2, -1.5),
            Vec3::new(1.0, 2.4, 1.0),
            Vec3::new(1.0, 0.22, 0.04),
        );
        spawn_cube(
            engine,
            "Green Tower",
            Vec3::new(2.2, 0.85, -2.3),
            Vec3::new(1.2, 1.7, 1.2),
            Vec3::new(0.05, 0.85, 0.28),
        );
        spawn_cube(
            engine,
            "Blue Marker",
            Vec3::new(0.0, 0.55, 2.2),
            Vec3::new(0.8, 1.1, 0.8),
            Vec3::new(0.06, 0.35, 1.0),
        );

        // Secondary camera: transform local -Z is its viewing direction. Game
        // code can update this transform every frame for a mirror, portal,
        // security camera, minimap, scope, or rear-view camera.
        let secondary_position = Vec3::new(4.4, 3.0, 3.8);
        let secondary_direction = (Vec3::new(0.0, 1.0, -0.8) - secondary_position).normalize();
        engine
            .spawn_actor("Portal View Camera")
            .with(Transform {
                translation: secondary_position,
                rotation: Quat::from_rotation_arc(Vec3::NEG_Z, secondary_direction),
                scale: Vec3::ONE,
            })
            .with(RenderTextureCamera {
                target_name: "portal_view".to_string(),
                width: 768,
                height: 512,
                // The shader surface is excluded automatically because it
                // samples this target. The layer mask is an additional generic
                // visibility control for games that need it.
                layer_mask: u32::MAX & !PORTAL_LAYER,
                clear_color: [0.015, 0.025, 0.05, 1.0],
                ..RenderTextureCamera::default()
            })
            .build();

        engine
            .spawn_actor("Portal Surface")
            .with(Transform {
                translation: Vec3::new(0.0, 1.65, 0.45),
                rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                scale: Vec3::new(3.8, 1.0, 2.35),
            })
            .bundle(RenderBundle {
                shape: Shape {
                    primitive: PrimitiveShape::Plane,
                    size: Vec3::new(1.0, 0.0, 1.0),
                },
                material: Material {
                    base_color: Vec3::ONE,
                    roughness: 0.0,
                    ..Material::default()
                },
                renderable: Renderable {
                    visible: true,
                    ..Renderable::default()
                },
            })
            .with(RenderLayers { mask: PORTAL_LAYER })
            .with(CustomShaderMaterial {
                shader_id: "examples/render_texture_portal".to_string(),
                wgsl_source: Some(PORTAL_SHADER.to_string()),
                vertex_interface: CustomShaderVertexInterface::Textured,
                render_textures: vec!["portal_view".to_string()],
                ..CustomShaderMaterial::default()
            })
            .build();

        engine
            .spawn_actor("Sun")
            .with(DirectionalLight {
                direction: Vec3::new(-0.45, -1.0, -0.25).normalize(),
                color: Vec3::new(1.0, 0.95, 0.86),
                intensity: 2.0,
                ..DirectionalLight::default()
            })
            .build();
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.time += dt.max(0.0);
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.time_seconds = self.time;
        }
        if engine
            .get_resource::<InputState>()
            .map(|input| input.quit_requested())
            .unwrap_or(false)
        {
            engine.stop();
        }
    }
}

fn spawn_cube(engine: &mut Engine, name: &str, position: Vec3, size: Vec3, color: Vec3) {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .bundle(RenderBundle {
            shape: Shape {
                primitive: PrimitiveShape::Cube,
                size,
            },
            material: Material {
                base_color: color,
                roughness: 0.48,
                ..Material::default()
            },
            renderable: Renderable {
                visible: true,
                ..Renderable::default()
            },
        })
        .build();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Render Texture Portal".to_string(),
            width: 1280,
            height: 720,
            clear_color: [0.01, 0.015, 0.028, 1.0],
            cursor_grab: false,
            cursor_visible: true,
            adapter_preference: AdapterPreference::HighPerformance,
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .run_until_stopped(
            RenderTexturePortalExample { time: 0.0 },
            None,
            1.0 / 60.0,
        )
}
