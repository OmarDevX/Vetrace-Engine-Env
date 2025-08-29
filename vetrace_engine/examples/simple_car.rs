use rapier3d::na::{UnitQuaternion, Vector3};
use rapier3d::prelude::*;

use sdl2::keyboard::Keycode;
use vetrace_engine::components::components::{
    Bloom, CameraAttachment, Collider, ColliderShape, DirectionalLight, Easing, FreeFlightControls,
    Lerp, LerpData, LerpState, LoopMode, Material, Parent, Particle, PostProcessing, Renderable,
    RigidBody3D, StaticBody, Timer, Transform,
};
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;
use vetrace_engine::Behaviour;

struct CarController {
    car: u32,
    velocity: [f32; 3],
    height_vel: f32,
    yaw: f32,
    wheel_angle: f32,
    bubble_timer: Timer,
}

impl CarController {
    fn new(car: u32) -> Self {
        Self {
            car,
            velocity: [0.0, 0.0, 0.0],
            height_vel: 0.0,
            yaw: 0.0,
            wheel_angle: 0.0,
            bubble_timer: Timer {
                wait_time: 0.05,
                autostart: true,
                ..Default::default()
            },
        }
    }
}

fn quat_from_yaw(yaw: f32) -> [f32; 4] {
    let h = yaw * 0.5;
    [0.0, h.sin(), 0.0, h.cos()]
}

fn unit_quat_from_yaw(yaw: f32) -> UnitQuaternion<f32> {
    UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw)
}

impl Behaviour for CarController {
    fn update(&mut self, engine: &mut Engine, dt: f32) {
        let forward_acc = 80.0;
        let steer_rate = 2.0; // wheel turn speed
        let wheel_return = 4.0;
        let max_wheel_angle = 0.2;
        let turn_factor = 2.0;
        let damping = 0.9;
        let spring_strength = 10.0;
        let spring_damping = 0.8;
        let target_height = 0.5;

        let mut forward = 0.0f32;
        let mut steer_input = 0.0f32;
        if engine.input.is_key_down(Keycode::W) {
            forward += 1.0;
        }
        if engine.input.is_key_down(Keycode::S) {
            forward -= 1.0;
        }
        if engine.input.is_key_down(Keycode::A) {
            steer_input += 1.0;
        }
        if engine.input.is_key_down(Keycode::D) {
            steer_input -= 1.0;
        }

        if let Some(ent) = engine.core.find_entity_by_object_id(self.car) {
            if let Some(trans) = engine.world.get_mut::<Transform>(ent) {
                // steer wheels
                self.wheel_angle += steer_input * steer_rate * dt;
                if steer_input.abs() < f32::EPSILON {
                    self.wheel_angle *= 1.0 - wheel_return * dt;
                }
                self.wheel_angle = self.wheel_angle.clamp(-max_wheel_angle, max_wheel_angle);

                let speed = (self.velocity[0] * self.velocity[0]
                    + self.velocity[2] * self.velocity[2])
                    .sqrt();
                let yaw_delta = self.wheel_angle * speed * turn_factor * dt;
                if yaw_delta.abs() > 0.0 {
                    let cosd = yaw_delta.cos();
                    let sind = yaw_delta.sin();
                    let vx = self.velocity[0] * cosd + self.velocity[2] * sind;
                    let vz = -self.velocity[0] * sind + self.velocity[2] * cosd;
                    self.velocity[0] = vx;
                    self.velocity[2] = vz;
                }

                self.yaw += yaw_delta;
                trans.orientation = quat_from_yaw(self.yaw);

                // forward direction assuming the car mesh faces +Z when yaw = 0
                let dir = [self.yaw.sin(), 0.0, self.yaw.cos()];
                self.velocity[0] += dir[0] * forward * forward_acc * dt;
                self.velocity[2] += dir[2] * forward * forward_acc * dt;
                self.velocity[0] *= damping;
                self.velocity[2] *= damping;
                trans.position[0] += self.velocity[0] * dt;
                trans.position[2] += self.velocity[2] * dt;

                let diff = target_height - trans.position[1];
                self.height_vel += diff * spring_strength * dt;
                self.height_vel *= spring_damping;
                trans.position[1] += self.height_vel * dt;

                // spawn cartoon bubble particles behind the car
                let speed = (self.velocity[0] * self.velocity[0]
                    + self.velocity[2] * self.velocity[2])
                    .sqrt();
                self.bubble_timer.paused = speed <= 0.1;
                if self.bubble_timer.tick(dt) {
                    let back_pos = [
                        trans.position[0] - dir[0] * 1.5,
                        trans.position[1],
                        trans.position[2] - dir[2] * 1.5,
                    ];
                    let ent = engine.spawn_empty("bubble");
                    engine.world.insert(
                        ent,
                        Transform {
                            position: back_pos,
                            orientation: [0.0, 0.0, 0.0, 1.0],
                            size: [0.4, 0.4, 0.4],
                        },
                    );
                    engine.world.insert(
                        ent,
                        Renderable {
                            color: [200.0, 200.0, 255.0],
                            roughness: 0.2,
                            emission: 0.0,
                            is_mesh: false,
                            triangle_start_idx: 0,
                            triangle_count: 0,
                        },
                    );
                    engine.world.insert(ent, Material::default());
                    engine.world.insert(
                        ent,
                        Collider {
                            shape: ColliderShape::Cube,
                            size: [0.4, 0.4, 0.4],
                            ..Default::default()
                        },
                    );
                    engine.world.insert(
                        ent,
                        Particle {
                            velocity: [0.0, 1.5, 0.0],
                            lifetime: 1.0,
                            start_size: 0.4,
                            end_size: 0.1,
                            looping: false,
                            initial_lifetime: 1.0,
                            initial_position: None,
                        },
                    );
                    engine.world.insert(
                        ent,
                        Lerp::F32(LerpData {
                            start: 0.4,
                            end: 0.1,
                            progress: 0.0,
                            speed: 1.0,
                            loop_mode: LoopMode::None,
                            state: LerpState::PlayingForward,
                            easing: Easing::Linear,
                        }),
                    );
                }
            }
            if let Some(rb) = engine.world.get_mut::<RigidBody3D>(ent) {
                if let Some(handle) = rb.handle {
                    if let Some(body) = engine.physics.bodies.get_mut(handle) {
                        let pos = engine
                            .world
                            .get::<Transform>(ent)
                            .map(|t| t.position)
                            .unwrap_or_default();
                        body.set_translation(vector![pos[0], pos[1], pos[2]], true);
                        body.set_rotation(unit_quat_from_yaw(self.yaw), true);
                        body.set_linvel(
                            vector![self.velocity[0], self.height_vel, self.velocity[2]],
                            true,
                        );
                    }
                }
            }
        }
    }
}

fn main() {
    let mut engine = Engine::new(false);

    // floor
    let mut floor = Object::default();
    floor.position = [0.0, -0.1, 0.0];
    floor.scale = [20.0, 0.2, 20.0];
    floor.is_cube = true;
    engine.spawn_object(floor);
    let floor_id = (engine.scene.objects.len() - 1) as u32;
    if let Some(ent) = engine.core.find_entity_by_object_id(floor_id) {
        engine.world.insert(ent, StaticBody::default());
    }

    // car body
    let mut car = Object::default();
    car.position = [0.0, 0.5, 0.0];
    car.scale = [1.5, 0.5, 3.0];
    car.color = [20.0, 200.0, 100.0];
    car.is_cube = true;
    engine.spawn_object(car);
    let car_id = (engine.scene.objects.len() - 1) as u32;
    if let Some(ent) = engine.core.find_entity_by_object_id(car_id) {
        let mut rb = RigidBody3D::default();
        rb.linear_damping = 4.0;
        rb.angular_damping = 4.0;
        rb.gravity_enabled = false;
        engine.world.insert(ent, rb);
    }

    // camera following the car
    let camera = engine.spawn_empty("camera");
    engine.world.insert(
        camera,
        Transform {
            position: [0.0, 2.0, -6.0],
            orientation: [0.0, 0.0, 0.0, 1.0],
            size: [1.0, 1.0, 1.0],
        },
    );
    let bloom = Bloom {
        threshold: 0.0,
        ..Default::default()
    };
    let ps = PostProcessing {
        bloom: Some(bloom),
        gi_enabled: false,
        ..Default::default()
    };
    let dl = DirectionalLight {
        color: [255.0, 255.0, 255.0],
        intensity: 10.0,
        direction: [-1.0, -1.0, -1.0],
    };
    engine.world.insert(camera, ps);
    engine.world.insert(camera, dl);
    engine.world.insert(camera, CameraAttachment::default());
    engine.world.insert(camera, FreeFlightControls::default());
    if let Some(car_ent) = engine.core.find_entity_by_object_id(car_id) {
        engine.world.insert(camera, Parent { entity: car_ent });
    }

    engine.add_behaviour(CarController::new(car_id));

    engine.run(true);
}
