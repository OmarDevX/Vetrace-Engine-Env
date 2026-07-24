//! Object-bounded raymarched cloud using `CustomShaderMaterial`.
//!
//! Run from the workspace root:
//! `cargo run -p vetrace_render --example raymarched_cloud --features wgpu_window`

use glam::{Quat, Vec3};
use vetrace_core::{App, AppBuilder, Engine, InputState, Transform};
use vetrace_render::{
    AdapterPreference, Camera, CustomShaderCullMode, CustomShaderDepthCompare,
    CustomShaderMaterial, CustomShaderRenderBucket, DirectionalLight, Material,
    PrimitiveShape, RenderBundle, RenderPlugin, RenderSettings, Renderable, Shape,
};

const CLOUD_SHADER: &str = include_str!("raymarched_cloud.wgsl");

struct RaymarchedCloudExample {
    time: f32,
}

impl App for RaymarchedCloudExample {
    fn setup(&mut self, engine: &mut Engine) {
        engine.insert_resource(Camera {
            position: Vec3::new(6.5, 3.8, 8.5),
            target: Vec3::new(0.0, 1.4, 0.0),
            fov_y_radians: 52.0_f32.to_radians(),
            ..Camera::default()
        });

        spawn_solid_cube(
            engine,
            "Ground",
            Vec3::new(0.0, -0.3, 0.0),
            Vec3::new(12.0, 0.4, 12.0),
            Vec3::new(0.055, 0.075, 0.11),
        );
        spawn_solid_cube(
            engine,
            "Object Behind Cloud",
            Vec3::new(-0.15, 1.25, -2.35),
            Vec3::new(1.15, 2.5, 1.15),
            Vec3::new(1.0, 0.24, 0.055),
        );
        spawn_solid_cube(
            engine,
            "Left Marker",
            Vec3::new(-3.0, 0.55, -0.8),
            Vec3::new(0.7, 1.1, 0.7),
            Vec3::new(0.08, 0.55, 0.95),
        );
        spawn_solid_cube(
            engine,
            "Right Marker",
            Vec3::new(3.0, 0.55, 0.4),
            Vec3::new(0.7, 1.1, 0.7),
            Vec3::new(0.15, 0.9, 0.45),
        );

        engine
            .spawn_actor("Raymarched Cloud Volume")
            .with(Transform {
                translation: Vec3::new(0.0, 1.55, 0.0),
                rotation: Quat::from_rotation_y(-0.12),
                scale: Vec3::new(5.2, 2.4, 3.2),
            })
            .bundle(RenderBundle {
                // The unit cube is the proxy volume. Its Transform scale controls
                // the cloud dimensions; the shader raymarches inside local ±0.5.
                shape: Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE },
                material: Material { base_color: Vec3::ONE, ..Material::default() },
                renderable: Renderable { visible: true, ..Renderable::default() },
            })
            .with(CustomShaderMaterial {
                shader_id: "examples/raymarched_cloud".to_string(),
                asset_path: None,
                wgsl_source: Some(CLOUD_SHADER.to_string()),
                // coverage, extinction, noise scale, wind speed
                params: vec![0.52, 7.5, 3.4, 0.16],
                fallback_color_a: Vec3::new(0.16, 0.24, 0.38),
                fallback_color_b: Vec3::new(1.0, 0.92, 0.78),
                // Draw the cube's back faces. Each fragment represents a ray
                // exiting the proxy volume.
                cull_mode: CustomShaderCullMode::Front,
                depth_write: false,
                depth_compare: CustomShaderDepthCompare::LessEqual,
                render_bucket: CustomShaderRenderBucket::Transparent,
                ..CustomShaderMaterial::default()
            })
            .build();

        engine
            .spawn_actor("Sun")
            .with(DirectionalLight {
                direction: Vec3::new(-0.55, -1.0, -0.35).normalize(),
                color: Vec3::new(1.0, 0.92, 0.78),
                intensity: 2.2,
                ..DirectionalLight::default()
            })
            .build();
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.time += dt;
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.time_seconds = self.time;
        }
        if engine.get_resource::<InputState>().map(|input| input.quit_requested()).unwrap_or(false) {
            engine.stop();
        }
    }
}

fn spawn_solid_cube(engine: &mut Engine, name: &str, position: Vec3, size: Vec3, color: Vec3) {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .bundle(RenderBundle {
            shape: Shape { primitive: PrimitiveShape::Cube, size },
            material: Material {
                base_color: color,
                roughness: 0.48,
                metallic: 0.05,
                ..Material::default()
            },
            renderable: Renderable { visible: true, ..Renderable::default() },
        })
        .build();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Raymarched Cloud".to_string(),
            width: 1280,
            height: 720,
            clear_color: [0.012, 0.022, 0.045, 1.0],
            cursor_grab: false,
            cursor_visible: true,
            // Same adapter policy used by Simple Shooter's --integrated-gpu.
            adapter_preference: AdapterPreference::LowPower,
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .run_until_stopped(RaymarchedCloudExample { time: 0.0 }, None, 1.0 / 60.0)
}
