//! 1:1 Earth horizon demonstration.
//!
//! One world unit is exactly one metre in this example. The Earth sphere uses
//! the mean real Earth radius, and the camera is a first-person player standing
//! on the surface. A small boat remains gravity-attached to the spherical
//! surface and continuously moves away so its lower half disappears first.
//!
//! Controls:
//! - Mouse: look around (captured automatically)
//! - F1: toggle captured mouse so the editor UI can be used for debugging
//! - W/A/S/D: walk along the curved Earth surface
//! - Left Shift: sprint
//! - R: reset the boat near the player
//! - Escape: quit

use glam::{Mat3, Quat, Vec3};
use sdl2::keyboard::Keycode;
use vetrace_editor::EditorPlugin;
use vetrace_engine::app::app;
use vetrace_engine::app::{App, InputEvent};
use vetrace_engine::components::components::{
    CameraAttachment, DirectionalLight, PostProcessing, Transform,
};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::scene::object::Object;

const EARTH_RADIUS_METRES: f32 = 6_371_000.0;
const PLAYER_EYE_HEIGHT_METRES: f32 = 1.8;
const WALK_SPEED_METRES_PER_SECOND: f32 = 6.0;
const SPRINT_SPEED_METRES_PER_SECOND: f32 = 35.0;
const MOUSE_SENSITIVITY_DEGREES_PER_PIXEL: f32 = 0.08;
const BOAT_START_DISTANCE_METRES: f32 = 500.0;
const BOAT_RESET_DISTANCE_METRES: f32 = 35_000.0;
// Faster than a real boat so the real-scale curvature demonstration happens quickly.
const BOAT_SPEED_METRES_PER_SECOND: f32 = 280.0;

struct TheEarthExample {
    camera: Option<Entity>,
    boat_hull: Option<Entity>,
    boat_mast: Option<Entity>,
    boat_flag: Option<Entity>,
    earth_center: Vec3,
    surface_normal: Vec3,
    heading: Vec3,
    pitch_degrees: f32,
    boat_distance_metres: f32,
    elapsed_seconds: f32,
    mouse_captured: bool,
}

impl Default for TheEarthExample {
    fn default() -> Self {
        Self {
            camera: None,
            boat_hull: None,
            boat_mast: None,
            boat_flag: None,
            earth_center: Vec3::new(0.0, -EARTH_RADIUS_METRES, 0.0),
            surface_normal: Vec3::Y,
            heading: Vec3::X,
            pitch_degrees: 0.0,
            boat_distance_metres: BOAT_START_DISTANCE_METRES,
            elapsed_seconds: 0.0,
            mouse_captured: true,
        }
    }
}

impl TheEarthExample {
    fn spawn_sphere(
        engine: &mut Engine,
        name: &str,
        position: Vec3,
        radius: f32,
        color: [f32; 3],
        roughness: f32,
        emission: f32,
    ) -> Entity {
        let mut object = Object::new(
            position.to_array(),
            radius,
            color,
            roughness,
            emission,
            true,
        );
        object.is_cube = false;
        let actor = engine
            .spawn_object_as_actor(object)
            .expect("spawned sphere should have an entity");
        let entity = actor.entity();
        drop(actor);
        if let Some(meta) = engine
            .world
            .get_mut::<vetrace_engine::components::components::Metadata>(entity)
        {
            meta.name = name.to_string();
        }
        entity
    }

    fn spawn_box(
        engine: &mut Engine,
        name: &str,
        position: Vec3,
        size: Vec3,
        color: [f32; 3],
        roughness: f32,
        emission: f32,
    ) -> Entity {
        let mut object = Object::new(position.to_array(), 1.0, color, roughness, emission, true);
        object.is_cube = true;
        object.size = size.to_array();
        let actor = engine
            .spawn_object_as_actor(object)
            .expect("spawned box should have an entity");
        let entity = actor.entity();
        drop(actor);
        if let Some(meta) = engine
            .world
            .get_mut::<vetrace_engine::components::components::Metadata>(entity)
        {
            meta.name = name.to_string();
        }
        entity
    }

    fn orientation_from_axes(forward: Vec3, up: Vec3) -> Quat {
        let f = forward.normalize();
        let u = up.normalize();
        let r = f.cross(u).normalize();
        Quat::from_mat3(&Mat3::from_cols(f, u, r)).normalize()
    }

    fn point_on_earth(&self, normal: Vec3, height_metres: f32) -> Vec3 {
        self.earth_center + normal.normalize() * (EARTH_RADIUS_METRES + height_metres)
    }

    fn boat_surface_normal(&self) -> Vec3 {
        // Move the boat along the great-circle route directly in front of the
        // starting player position. This is real arc distance: theta = metres / radius.
        let theta = self.boat_distance_metres / EARTH_RADIUS_METRES;
        (Vec3::Y * theta.cos() + Vec3::X * theta.sin()).normalize()
    }

    fn update_camera_transform(&mut self, engine: &mut Engine) {
        let Some(camera) = self.camera else {
            return;
        };

        let up = self.surface_normal.normalize();
        self.heading = (self.heading - up * self.heading.dot(up)).normalize_or_zero();
        if self.heading.length_squared() < 0.5 {
            self.heading = Vec3::X;
        }

        let pitch = self.pitch_degrees.to_radians();
        let front = (self.heading * pitch.cos() + up * pitch.sin()).normalize();
        let orientation = Self::orientation_from_axes(front, up);
        let position = self.point_on_earth(up, PLAYER_EYE_HEIGHT_METRES);

        if let Some(transform) = engine.world.get_mut::<Transform>(camera) {
            transform.position = position.to_array();
            transform.orientation = orientation.to_array();
        }
    }

    fn update_boat_transforms(&mut self, engine: &mut Engine) {
        let normal = self.boat_surface_normal();
        let tangent = (Vec3::X - normal * Vec3::X.dot(normal)).normalize();
        let orientation = Self::orientation_from_axes(tangent, normal).to_array();

        let hull_size = Vec3::new(12.0, 3.0, 4.0);
        let mast_size = Vec3::new(0.45, 18.0, 0.45);
        let flag_size = Vec3::new(4.0, 2.2, 0.25);

        if let Some(entity) = self.boat_hull {
            if let Some(transform) = engine.world.get_mut::<Transform>(entity) {
                transform.position = self.point_on_earth(normal, hull_size.y * 0.5).to_array();
                transform.orientation = orientation;
                transform.size = hull_size.to_array();
            }
        }
        if let Some(entity) = self.boat_mast {
            if let Some(transform) = engine.world.get_mut::<Transform>(entity) {
                transform.position = self
                    .point_on_earth(normal, hull_size.y + mast_size.y * 0.5)
                    .to_array();
                transform.orientation = orientation;
                transform.size = mast_size.to_array();
            }
        }
        if let Some(entity) = self.boat_flag {
            if let Some(transform) = engine.world.get_mut::<Transform>(entity) {
                transform.position = self.point_on_earth(normal, 16.0).to_array();
                transform.orientation = orientation;
                transform.size = flag_size.to_array();
            }
        }
    }

    fn move_player_on_sphere(&mut self, local_motion: Vec3) {
        if local_motion.length_squared() == 0.0 {
            return;
        }

        let up = self.surface_normal.normalize();
        let right = self.heading.cross(up).normalize();
        let tangent_motion = self.heading * local_motion.x + right * local_motion.z;
        if tangent_motion.length_squared() == 0.0 {
            return;
        }

        // For small per-frame walking distances, normalizing this offset is a
        // stable approximation of moving over the real-radius sphere by metres.
        let surface_offset = tangent_motion / EARTH_RADIUS_METRES;
        self.surface_normal = (self.surface_normal + surface_offset).normalize();
        self.heading = (self.heading - self.surface_normal * self.heading.dot(self.surface_normal))
            .normalize();
    }
}

impl App for TheEarthExample {
    fn setup(&mut self, engine: &mut Engine) {
        engine.sky_color = [92.0, 148.0, 220.0];
        engine.capture_mouse(self.mouse_captured);

        println!("The Earth example: 1 unit = 1 metre.");
        println!(
            "Earth radius: {EARTH_RADIUS_METRES} m. Eye height: {PLAYER_EYE_HEIGHT_METRES} m."
        );
        println!(
            "Mouse is captured. Use WASD to walk, Left Shift to sprint, F1 to toggle editor mouse, R to reset the boat, Escape to quit."
        );

        Self::spawn_sphere(
            engine,
            "the earth (1:1 scale, radius 6,371,000 m)",
            self.earth_center,
            EARTH_RADIUS_METRES,
            [0.05, 0.28, 0.08],
            0.82,
            0.0,
        );

        // A bright, real-size reference point directly at the player start.
        Self::spawn_box(
            engine,
            "one metre surface marker",
            self.point_on_earth(Vec3::Y, 0.5),
            Vec3::splat(1.0),
            [1.0, 0.95, 0.2],
            0.35,
            0.0,
        );

        self.boat_hull = Some(Self::spawn_box(
            engine,
            "gravity-attached boat hull",
            self.point_on_earth(self.boat_surface_normal(), 1.5),
            Vec3::new(12.0, 3.0, 4.0),
            [0.72, 0.18, 0.08],
            0.45,
            0.0,
        ));
        self.boat_mast = Some(Self::spawn_box(
            engine,
            "gravity-attached boat mast",
            self.point_on_earth(self.boat_surface_normal(), 12.0),
            Vec3::new(0.45, 18.0, 0.45),
            [0.92, 0.86, 0.62],
            0.55,
            0.0,
        ));
        self.boat_flag = Some(Self::spawn_box(
            engine,
            "high flag visible after hull disappears",
            self.point_on_earth(self.boat_surface_normal(), 16.0),
            Vec3::new(4.0, 2.2, 0.25),
            [1.0, 1.0, 1.0],
            0.25,
            0.0,
        ));

        let camera = engine.spawn_empty("first person player camera");
        engine.world.insert(camera, Transform::default());
        engine.world.insert(
            camera,
            CameraAttachment {
                fov: 65.0_f32.to_radians(),
                local_offset: [0.0, 0.0, 0.0],
            },
        );
        engine.world.insert(
            camera,
            PostProcessing {
                // Keep temporal denoising active and bias it toward history so
                // the large, low-sample ray-traced scene does not shimmer.
                temporal_blend: 0.12,
                gi_temporal_blend: 0.9,
                history_clamp_k: 2.0,
                light_samples: 4,
                dir_light_samples: 4,
                ..Default::default()
            },
        );
        self.camera = Some(camera);

        let light = engine.spawn_empty("sun directional light");
        engine.world.insert(
            light,
            DirectionalLight {
                direction: [-0.35, -1.0, -0.25],
                color: [255.0, 248.0, 232.0],
                intensity: 4.0,
            },
        );

        self.update_camera_transform(engine);
        self.update_boat_transforms(engine);
    }

    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        self.elapsed_seconds += delta_time;

        let up = self.surface_normal.normalize();
        if self.mouse_captured {
            let (mouse_dx, mouse_dy) = engine.input.mouse_delta();
            let yaw_delta = -(mouse_dx as f32) * MOUSE_SENSITIVITY_DEGREES_PER_PIXEL;
            if yaw_delta != 0.0 {
                self.heading =
                    (Quat::from_axis_angle(up, yaw_delta.to_radians()) * self.heading).normalize();
            }
            self.pitch_degrees = (self.pitch_degrees
                - mouse_dy as f32 * MOUSE_SENSITIVITY_DEGREES_PER_PIXEL)
                .clamp(-88.0, 88.0);
        }
        self.heading = (self.heading - up * self.heading.dot(up)).normalize();

        let mut input = Vec3::ZERO;
        if engine.input.is_key_down(Keycode::W) {
            input.x += 1.0;
        }
        if engine.input.is_key_down(Keycode::S) {
            input.x -= 1.0;
        }
        if engine.input.is_key_down(Keycode::D) {
            input.z += 1.0;
        }
        if engine.input.is_key_down(Keycode::A) {
            input.z -= 1.0;
        }
        if input.length_squared() > 0.0 {
            input = input.normalize();
        }
        let speed = if engine.input.is_key_down(Keycode::LShift) {
            SPRINT_SPEED_METRES_PER_SECOND
        } else {
            WALK_SPEED_METRES_PER_SECOND
        };
        self.move_player_on_sphere(input * speed * delta_time);

        self.boat_distance_metres += BOAT_SPEED_METRES_PER_SECOND * delta_time;
        if self.boat_distance_metres > BOAT_RESET_DISTANCE_METRES {
            self.boat_distance_metres = BOAT_START_DISTANCE_METRES;
        }

        if (self.elapsed_seconds % 5.0) < delta_time {
            let horizon = (2.0 * EARTH_RADIUS_METRES * PLAYER_EYE_HEIGHT_METRES).sqrt();
            println!(
                "Boat arc distance: {:.0} m. Eye-level geometric horizon: {:.0} m.",
                self.boat_distance_metres, horizon
            );
        }

        self.update_camera_transform(engine);
        self.update_boat_transforms(engine);
    }

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }

    fn cleanup(&mut self, engine: &mut Engine) {
        engine.capture_mouse(false);
    }

    fn on_input(&mut self, _engine: &mut Engine, event: &InputEvent) {
        if let InputEvent::KeyPressed { key } = event {
            if *key == Keycode::F1 {
                self.mouse_captured = !self.mouse_captured;
                _engine.capture_mouse(self.mouse_captured);
                println!(
                    "Mouse capture {}. {}",
                    if self.mouse_captured {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    if self.mouse_captured {
                        "First-person look is active."
                    } else {
                        "Use the editor UI for debugging."
                    }
                );
            } else if *key == Keycode::R {
                self.boat_distance_metres = BOAT_START_DISTANCE_METRES;
                println!("Boat reset to {BOAT_START_DISTANCE_METRES} m away.");
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("The Earth - 1:1 Scale Horizon Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .add_plugin(EditorPlugin::new())
        .run(TheEarthExample::default())?;
    Ok(())
}
