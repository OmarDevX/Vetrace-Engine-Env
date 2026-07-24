use glam::{Quat, Vec3};
use vetrace_core::components::builtins::Transform;
use vetrace_core::{Actor, Engine, InputState, Stage};
use vetrace_render::{
    AntiAliasingMode, Camera, DirectionalLight, Material, PointLight, PrimitiveShape,
    RenderSettings, Renderable, ShadowMode, Shape,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExampleKind {
    RotatingCube,
    Shapes,
    Materials,
    Lighting,
    ManyCubes,
    Hierarchy,
    CameraControls,
}

impl ExampleKind {
    pub fn from_slug(slug: &str) -> Self {
        match slug {
            "shapes" => Self::Shapes,
            "materials" => Self::Materials,
            "lighting" => Self::Lighting,
            "many-cubes" => Self::ManyCubes,
            "hierarchy" => Self::Hierarchy,
            "camera-controls" => Self::CameraControls,
            _ => Self::RotatingCube,
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::RotatingCube => "rotating-cube",
            Self::Shapes => "shapes",
            Self::Materials => "materials",
            Self::Lighting => "lighting",
            Self::ManyCubes => "many-cubes",
            Self::Hierarchy => "hierarchy",
            Self::CameraControls => "camera-controls",
        }
    }
}

pub struct ExampleScene {
    kind: ExampleKind,
    animated: Vec<Actor>,
    base_positions: Vec<Vec3>,
    point_lights: Vec<Actor>,
    orbit_yaw: f32,
    orbit_pitch: f32,
    orbit_distance: f32,
}

impl ExampleScene {
    pub fn build(kind: ExampleKind) -> Result<(Engine, Self), String> {
        let mut engine = Engine::new();
        let mut settings = RenderSettings::default();
        settings.title = format!("Vetrace Web · {}", kind.slug());
        settings.clear_color = match kind {
            ExampleKind::Lighting => [0.018, 0.026, 0.055, 1.0],
            ExampleKind::ManyCubes => [0.025, 0.035, 0.065, 1.0],
            _ => [0.035, 0.050, 0.090, 1.0],
        };
        settings.draw_bounds = false;
        // Keep the first browser frame on the direct surface path. This makes
        // the examples a reliable renderer smoke test before optional
        // post-processing is enabled by a game or project.
        settings.anti_aliasing_mode = AntiAliasingMode::Off;
        settings.cursor_grab = false;
        settings.cursor_visible = true;
        engine.insert_resource(settings);
        engine.insert_resource(Camera {
            position: Vec3::new(7.0, 5.0, 9.0),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        });

        spawn_directional_light(&mut engine);
        let mut scene = Self {
            kind,
            animated: Vec::new(),
            base_positions: Vec::new(),
            point_lights: Vec::new(),
            orbit_yaw: 0.66,
            orbit_pitch: 0.38,
            orbit_distance: 11.5,
        };

        match kind {
            ExampleKind::RotatingCube => scene.build_rotating_cube(&mut engine),
            ExampleKind::Shapes => scene.build_shapes(&mut engine),
            ExampleKind::Materials => scene.build_materials(&mut engine),
            ExampleKind::Lighting => scene.build_lighting(&mut engine),
            ExampleKind::ManyCubes => scene.build_many_cubes(&mut engine),
            ExampleKind::Hierarchy => scene.build_hierarchy(&mut engine)?,
            ExampleKind::CameraControls => scene.build_camera_controls(&mut engine),
        }

        engine.run_stage(Stage::RenderExtract, 0.0);
        Ok((engine, scene))
    }

    pub fn update(&mut self, engine: &mut Engine, time: f32, dt: f32) {
        self.update_camera(engine, dt);
        match self.kind {
            ExampleKind::RotatingCube => {
                if let Some(actor) = self.animated.first().copied() {
                    let _ = actor.set_rotation(
                        engine,
                        Quat::from_rotation_y(time * 0.8) * Quat::from_rotation_x(time * 0.32),
                    );
                }
            }
            ExampleKind::Shapes | ExampleKind::Materials => {
                for (index, actor) in self.animated.iter().copied().enumerate() {
                    let phase = index as f32 * 0.37;
                    let _ = actor.set_rotation(
                        engine,
                        Quat::from_rotation_y(time * (0.35 + index as f32 * 0.025) + phase),
                    );
                }
            }
            ExampleKind::Lighting => {
                for (index, light) in self.point_lights.iter().copied().enumerate() {
                    let phase = index as f32 * std::f32::consts::TAU / self.point_lights.len().max(1) as f32;
                    let radius = 3.2 + index as f32 * 0.25;
                    let position = Vec3::new(
                        (time * 0.65 + phase).cos() * radius,
                        1.8 + (time * 1.2 + phase).sin() * 0.7,
                        (time * 0.65 + phase).sin() * radius,
                    );
                    let _ = light.set_position(engine, position);
                }
                for (index, actor) in self.animated.iter().copied().enumerate() {
                    let _ = actor.set_rotation(engine, Quat::from_rotation_y(time * 0.28 + index as f32));
                }
            }
            ExampleKind::ManyCubes => {
                for (index, actor) in self.animated.iter().copied().enumerate() {
                    let base = self.base_positions[index];
                    let wave = (time * 2.0 + base.x * 0.65 + base.z * 0.48).sin();
                    let _ = actor.set_position(engine, Vec3::new(base.x, base.y + wave * 0.42, base.z));
                    let _ = actor.set_rotation(engine, Quat::from_rotation_y(time * 0.22 + wave * 0.25));
                }
            }
            ExampleKind::Hierarchy => {
                if let Some(parent) = self.animated.first().copied() {
                    let _ = parent.set_rotation(engine, Quat::from_rotation_y(time * 0.55));
                }
                for (index, actor) in self.animated.iter().copied().enumerate().skip(1) {
                    let local_spin = Quat::from_rotation_x(time * (0.6 + index as f32 * 0.08));
                    let _ = actor.set_rotation(engine, local_spin);
                }
            }
            ExampleKind::CameraControls => {
                for (index, actor) in self.animated.iter().copied().enumerate() {
                    let base = self.base_positions[index];
                    let y = base.y + (time * 1.1 + index as f32 * 0.8).sin() * 0.2;
                    let _ = actor.set_position(engine, Vec3::new(base.x, y, base.z));
                }
            }
        }
    }

    fn update_camera(&mut self, engine: &mut Engine, dt: f32) {
        let (mouse_delta, wheel, drag, left, right, up, down) = engine
            .get_resource::<InputState>()
            .map(|input| {
                (
                    input.mouse_delta(),
                    input.mouse_wheel_delta(),
                    input.is_mouse_button_down("Left"),
                    input.is_key_down("ArrowLeft") || input.is_key_down("KeyA"),
                    input.is_key_down("ArrowRight") || input.is_key_down("KeyD"),
                    input.is_key_down("ArrowUp") || input.is_key_down("KeyW"),
                    input.is_key_down("ArrowDown") || input.is_key_down("KeyS"),
                )
            })
            .unwrap_or_default();

        if drag {
            self.orbit_yaw -= mouse_delta.0 * 0.006;
            self.orbit_pitch = (self.orbit_pitch + mouse_delta.1 * 0.006).clamp(-1.15, 1.15);
        }
        let keyboard_speed = 1.45 * dt;
        if left { self.orbit_yaw += keyboard_speed; }
        if right { self.orbit_yaw -= keyboard_speed; }
        if up { self.orbit_pitch = (self.orbit_pitch + keyboard_speed).clamp(-1.15, 1.15); }
        if down { self.orbit_pitch = (self.orbit_pitch - keyboard_speed).clamp(-1.15, 1.15); }
        self.orbit_distance = (self.orbit_distance + wheel.1 * 0.008).clamp(4.0, 28.0);

        let target = match self.kind {
            ExampleKind::ManyCubes => Vec3::new(0.0, 0.3, 0.0),
            _ => Vec3::new(0.0, 0.8, 0.0),
        };
        let horizontal = self.orbit_distance * self.orbit_pitch.cos();
        let position = target
            + Vec3::new(
                self.orbit_yaw.sin() * horizontal,
                self.orbit_distance * self.orbit_pitch.sin(),
                self.orbit_yaw.cos() * horizontal,
            );
        if let Some(camera) = engine.get_resource_mut::<Camera>() {
            camera.position = position;
            camera.target = target;
        }
    }

    fn build_rotating_cube(&mut self, engine: &mut Engine) {
        spawn_floor(engine, 12.0);
        let cube = spawn_shape(
            engine,
            "Rotating Cube",
            PrimitiveShape::Cube,
            Vec3::new(0.0, 1.2, 0.0),
            Vec3::splat(2.15),
            Vec3::new(0.16, 0.68, 1.0),
            0.24,
            0.12,
            Vec3::ZERO,
        );
        self.animated.push(cube);
    }

    fn build_shapes(&mut self, engine: &mut Engine) {
        spawn_floor(engine, 16.0);
        let shapes = [
            (PrimitiveShape::Cube, Vec3::new(-3.6, 1.0, 0.0), Vec3::splat(1.7), Vec3::new(0.12, 0.65, 0.98)),
            (PrimitiveShape::Sphere, Vec3::new(-1.2, 1.0, 0.0), Vec3::splat(1.8), Vec3::new(0.75, 0.25, 1.0)),
            (PrimitiveShape::Capsule, Vec3::new(1.35, 1.25, 0.0), Vec3::new(1.4, 2.5, 1.4), Vec3::new(0.2, 0.9, 0.58)),
            (PrimitiveShape::Cube, Vec3::new(3.8, 0.8, 0.0), Vec3::new(1.8, 1.2, 1.8), Vec3::new(1.0, 0.48, 0.16)),
        ];
        for (index, (shape, position, size, color)) in shapes.into_iter().enumerate() {
            self.animated.push(spawn_shape(
                engine,
                &format!("Shape {index}"),
                shape,
                position,
                size,
                color,
                0.35,
                0.08,
                Vec3::ZERO,
            ));
        }
    }

    fn build_materials(&mut self, engine: &mut Engine) {
        spawn_floor(engine, 17.0);
        for index in 0..7 {
            let metallic = index as f32 / 6.0;
            let roughness = 0.08 + (1.0 - metallic) * 0.78;
            let x = (index as f32 - 3.0) * 1.65;
            let color = Vec3::new(0.12 + metallic * 0.7, 0.42, 0.96 - metallic * 0.45);
            self.animated.push(spawn_shape(
                engine,
                &format!("Material {index}"),
                PrimitiveShape::Sphere,
                Vec3::new(x, 1.0, 0.0),
                Vec3::splat(1.45),
                color,
                roughness,
                metallic,
                Vec3::ZERO,
            ));
        }
    }

    fn build_lighting(&mut self, engine: &mut Engine) {
        spawn_floor(engine, 15.0);
        for x in -2_i32..=2_i32 {
            let color = Vec3::new(0.18 + (x + 2) as f32 * 0.09, 0.28, 0.38);
            self.animated.push(spawn_shape(
                engine,
                &format!("Lit Column {x}"),
                PrimitiveShape::Cube,
                Vec3::new(x as f32 * 1.7, 1.0, 0.0),
                Vec3::new(1.1, 2.0 + (x.abs() as f32 * 0.28), 1.1),
                color,
                0.32,
                0.2,
                Vec3::ZERO,
            ));
        }
        let colors = [
            Vec3::new(1.0, 0.16, 0.08),
            Vec3::new(0.08, 0.45, 1.0),
            Vec3::new(0.18, 1.0, 0.42),
        ];
        for (index, color) in colors.into_iter().enumerate() {
            let light = spawn_shape(
                engine,
                &format!("Point Light {index}"),
                PrimitiveShape::Sphere,
                Vec3::new(index as f32 - 1.0, 2.0, 2.0),
                Vec3::splat(0.34),
                color,
                0.15,
                0.0,
                color * 2.6,
            );
            let _ = light.insert(
                engine,
                PointLight {
                    color,
                    intensity: 8.0,
                    range: Some(7.5),
                    shadow_mode: ShadowMode::None,
                },
            );
            self.point_lights.push(light);
        }
    }

    fn build_many_cubes(&mut self, engine: &mut Engine) {
        spawn_floor(engine, 22.0);
        for z in -6..=6 {
            for x in -6..=6 {
                let position = Vec3::new(x as f32 * 1.05, 0.5, z as f32 * 1.05);
                let distance = Vec3::new(x as f32, 0.0, z as f32).length() / 8.5;
                let color = Vec3::new(0.08 + distance * 0.28, 0.5 + (1.0 - distance) * 0.25, 0.9);
                let actor = spawn_shape(
                    engine,
                    &format!("Cube {x} {z}"),
                    PrimitiveShape::Cube,
                    position,
                    Vec3::splat(0.74),
                    color,
                    0.46,
                    0.04,
                    Vec3::ZERO,
                );
                self.animated.push(actor);
                self.base_positions.push(position);
            }
        }
        self.orbit_distance = 17.5;
        self.orbit_pitch = 0.58;
    }

    fn build_hierarchy(&mut self, engine: &mut Engine) -> Result<(), String> {
        spawn_floor(engine, 15.0);
        let parent = spawn_shape(
            engine,
            "Hierarchy Root",
            PrimitiveShape::Cube,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::splat(1.6),
            Vec3::new(1.0, 0.45, 0.1),
            0.3,
            0.1,
            Vec3::ZERO,
        );
        self.animated.push(parent);
        for index in 0..5 {
            let angle = index as f32 / 5.0 * std::f32::consts::TAU;
            let child = engine
                .spawn_actor(format!("Hierarchy Child {index}"))
                .with(Transform {
                    translation: Vec3::new(angle.cos() * 3.2, 0.25 + index as f32 * 0.12, angle.sin() * 3.2),
                    scale: Vec3::ONE,
                    rotation: Quat::IDENTITY,
                })
                .with(Shape { primitive: PrimitiveShape::Sphere, size: Vec3::splat(0.9) })
                .with(Material {
                    base_color: Vec3::new(0.18, 0.45 + index as f32 * 0.08, 1.0 - index as f32 * 0.1),
                    roughness: 0.25,
                    metallic: 0.18,
                    ..Material::default()
                })
                .with(Renderable { mesh: None, material: None, visible: true })
                .child_of(parent)
                .map_err(|error| error.to_string())?
                .try_build()
                .map_err(|error| error.to_string())?;
            self.animated.push(child);
        }
        Ok(())
    }

    fn build_camera_controls(&mut self, engine: &mut Engine) {
        spawn_floor(engine, 18.0);
        for index in 0..10 {
            let angle = index as f32 / 10.0 * std::f32::consts::TAU;
            let position = Vec3::new(angle.cos() * 4.0, 0.7, angle.sin() * 4.0);
            let actor = spawn_shape(
                engine,
                &format!("Orbit Target {index}"),
                if index % 2 == 0 { PrimitiveShape::Cube } else { PrimitiveShape::Sphere },
                position,
                Vec3::splat(1.0),
                Vec3::new(0.18 + index as f32 * 0.06, 0.68, 0.94 - index as f32 * 0.045),
                0.38,
                0.08,
                Vec3::ZERO,
            );
            self.animated.push(actor);
            self.base_positions.push(position);
        }
    }
}

fn spawn_directional_light(engine: &mut Engine) {
    engine
        .spawn_actor("Web Sun")
        .with(DirectionalLight {
            direction: Vec3::new(-0.45, -1.0, -0.3).normalize(),
            color: Vec3::new(1.0, 0.96, 0.88),
            intensity: 1.5,
            shadow_mode: ShadowMode::None,
        })
        .build();
}

fn spawn_floor(engine: &mut Engine, size: f32) -> Actor {
    spawn_shape(
        engine,
        "Floor",
        PrimitiveShape::Plane,
        Vec3::ZERO,
        Vec3::new(size, 1.0, size),
        Vec3::new(0.075, 0.085, 0.12),
        0.78,
        0.02,
        Vec3::ZERO,
    )
}

#[allow(clippy::too_many_arguments)]
fn spawn_shape(
    engine: &mut Engine,
    name: &str,
    primitive: PrimitiveShape,
    position: Vec3,
    size: Vec3,
    color: Vec3,
    roughness: f32,
    metallic: f32,
    emissive: Vec3,
) -> Actor {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .with(Shape { primitive, size })
        .with(Material {
            base_color: color,
            emissive,
            roughness,
            metallic,
            ..Material::default()
        })
        .with(Renderable { mesh: None, material: None, visible: true })
        .build()
}
