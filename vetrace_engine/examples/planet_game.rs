use glam::{Quat, Vec3};
use rapier3d::na::{Quaternion as RapierQuat, UnitQuaternion};
use rapier3d::prelude::*;
use sdl2::keyboard::Keycode;
use vetrace_engine::app::{app, App};
use vetrace_engine::components::components::{
    Atmosphere, CameraAttachment, Collider, RigidBody3D, StaticBody, Transform,
};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;
use vetrace_editor::EditorPlugin;

const G: f32 = 0.1;
const THRUST: f32 = 20.0;
const ROT_SPEED: f32 = 1.5;
const THROTTLE_RATE: f32 = 3.0;
const MAX_THROTTLE: f32 = 5.0;
const MAX_SPEED: f32 = 150.0;
const FLOATING_ORIGIN_THRESHOLD: f32 = 1000.0;

struct Planet {
    entity: Entity,
    atmosphere: Entity,
    mass: f32,
}

struct Player {
    entity: Entity,
    mass: f32,
    throttle: f32,
    orientation: Quat,
}

impl Default for Player {
    fn default() -> Self {
        Self { entity: Entity(0), mass: 10.0, throttle: 0.0, orientation: Quat::IDENTITY }
    }
}

struct PlanetGame {
    planets: Vec<Planet>,
    player: Player,
    camera: Entity,
}

impl Default for PlanetGame {
    fn default() -> Self {
        Self { planets: Vec::new(), player: Player::default(), camera: Entity(0) }
    }
}

impl PlanetGame {
    fn spawn_planet(
        &mut self,
        engine: &mut Engine,
        position: Vec3,
        radius: f32,
        mass: f32,
        color: [f32; 3],
    ) {
        let mut planet = Object::default();
        planet.is_cube = false;
        planet.radius = radius;
        planet.position = position.to_array();
        planet.color = color;
        let actor = engine.spawn_object_as_actor(planet).expect("spawn planet");
        let entity = actor.entity();
        engine.world.insert(entity, StaticBody::default());

        let mut atmosphere = Object::default();
        atmosphere.is_cube = false;
        atmosphere.radius = radius * 1.1;
        atmosphere.position = position.to_array();
        atmosphere.color = [color[0] * 0.5, color[1] * 0.5, color[2] * 0.5];
        atmosphere.is_glass = true;
        let atm_actor = engine
            .spawn_object_as_actor(atmosphere)
            .expect("spawn atmosphere");
        let atmosphere_entity = atm_actor.entity();
        engine.world.remove::<Collider>(atmosphere_entity);
        engine.world.insert(
            atmosphere_entity,
            Atmosphere {
                planet_radius: radius,
                atmo_radius: radius * 1.1,
                ..Default::default()
            },
        );

        self.planets.push(Planet {
            entity,
            atmosphere: atmosphere_entity,
            mass,
        });
    }

    fn shift_world(&mut self, engine: &mut Engine, offset: Vec3) {
        for planet in &self.planets {
            if let Some(t) = engine.world.get_mut::<Transform>(planet.entity) {
                let pos = Vec3::from_array(t.position) - offset;
                t.position = pos.to_array();
            }
            if let Some(sb) = engine.world.get::<StaticBody>(planet.entity) {
                if let Some(handle) = sb.handle {
                    if let Some(body) = engine.physics.bodies.get_mut(handle) {
                        let p = body.translation();
                        body.set_translation(
                            vector![p.x - offset.x, p.y - offset.y, p.z - offset.z],
                            true,
                        );
                    }
                }
            }
            if let Some(at) = engine.world.get_mut::<Transform>(planet.atmosphere) {
                let pos = Vec3::from_array(at.position) - offset;
                at.position = pos.to_array();
            }
        }

        if let Some(t) = engine.world.get_mut::<Transform>(self.player.entity) {
            let pos = Vec3::from_array(t.position) - offset;
            t.position = pos.to_array();
        }
        if let Some(rb) = engine.world.get::<RigidBody3D>(self.player.entity) {
            if let Some(handle) = rb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let p = body.translation();
                    body.set_translation(
                        vector![p.x - offset.x, p.y - offset.y, p.z - offset.z],
                        true,
                    );
                }
            }
        }
        if let Some(cam_t) = engine.world.get_mut::<Transform>(self.camera) {
            let pos = Vec3::from_array(cam_t.position) - offset;
            cam_t.position = pos.to_array();
        }
    }
}

impl App for PlanetGame {
    fn setup(&mut self, engine: &mut Engine) {
        self.spawn_planet(
            engine,
            Vec3::new(-150.0, 0.0, 0.0),
            80.0,
            1000.0,
            [0.0, 64.0, 12.0],
        );
        self.spawn_planet(
            engine,
            Vec3::new(150.0, 0.0, 0.0),
            40.0,
            100.0,
            [64.0, 16.0, 64.0],
        );

        let mut ship = Object::default();
        ship.is_cube = true;
        ship.size = [2.0, 1.0, 4.0];
        ship.position = [0.0, 0.0, 50.0];
        ship.color = [255.0, 255.0, 255.0];
        ship.mass = self.player.mass;
        let actor = engine.spawn_object_as_actor(ship).expect("spawn ship");
        self.player.entity = actor.entity();
        engine.world.insert(
            self.player.entity,
            RigidBody3D { gravity_enabled: false, mass: self.player.mass, ..Default::default() },
        );

        let cam = engine.spawn_empty("camera");
        engine.world.insert(cam, Transform { position: [0.0, 2.0, -10.0], ..Default::default() });
        engine.world.insert(cam, CameraAttachment::default());
        self.camera = cam;

        let mut sun = Object::default();
        sun.is_cube = false;
        sun.radius = 200.0;
        sun.position = [0.0, 0.0, -500.0];
        sun.color = [255.0, 255.0, 200.0];
        sun.emission = 10.0;
        engine.spawn_object(sun);
    }

    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let input = &engine.input;
        if input.is_key_down(Keycode::W) {
            self.player.orientation *= Quat::from_rotation_z(ROT_SPEED * delta);
        }
        if input.is_key_down(Keycode::S) {
            self.player.orientation *= Quat::from_rotation_z(-ROT_SPEED * delta);
        }
        if input.is_key_down(Keycode::A) {
            self.player.orientation *= Quat::from_rotation_y(ROT_SPEED * delta);
        }
        if input.is_key_down(Keycode::D) {
            self.player.orientation *= Quat::from_rotation_y(-ROT_SPEED * delta);
        }
        if input.is_key_down(Keycode::E) {
            self.player.throttle =
                (self.player.throttle + THROTTLE_RATE * delta).clamp(0.0, MAX_THROTTLE);
        }
        if input.is_key_down(Keycode::Q) {
            self.player.throttle =
                (self.player.throttle - THROTTLE_RATE * delta).clamp(0.0, MAX_THROTTLE);
        }
        self.player.orientation = self.player.orientation.normalize();

        if let Some(t) = engine.world.get_mut::<Transform>(self.player.entity) {
            t.orientation = [
                self.player.orientation.x,
                self.player.orientation.y,
                self.player.orientation.z,
                self.player.orientation.w,
            ];
        }

        if let Some(rb) = engine.world.get::<RigidBody3D>(self.player.entity) {
            if let Some(handle) = rb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let rot = UnitQuaternion::from_quaternion(RapierQuat::new(
                        self.player.orientation.w,
                        self.player.orientation.x,
                        self.player.orientation.y,
                        self.player.orientation.z,
                    ));
                    body.set_rotation(rot, true);

                    let forward = self.player.orientation * Vec3::X;
                    let thrust = forward * (THRUST * self.player.throttle);
                    body.add_force(vector![thrust.x, thrust.y, thrust.z], true);

                    if let Some(t) = engine.world.get::<Transform>(self.player.entity) {
                        let ship_pos = Vec3::from_array(t.position);
                        for planet in &self.planets {
                            if let Some(pt) = engine.world.get::<Transform>(planet.entity) {
                                let dir = Vec3::from_array(pt.position) - ship_pos;
                                let dist_sq = dir.length_squared().max(1.0);
                                let force_mag = G * self.player.mass * planet.mass / dist_sq;
                                let force = dir.normalize() * force_mag;
                                body.add_force(vector![force.x, force.y, force.z], true);
                            }
                        }
                    }

                    let vel = body.linvel();
                    let speed = vel.norm();
                    if speed > MAX_SPEED {
                        let clamped = vel / speed * MAX_SPEED;
                        body.set_linvel(clamped, true);
                    }
                }
            }
        }

        if let (Some(ship_t), Some(cam_t)) = (
            engine.world.get::<Transform>(self.player.entity).cloned(),
            engine.world.get_mut::<Transform>(self.camera),
        ) {
            let ship_pos = Vec3::from_array(ship_t.position);
            let offset = self.player.orientation * Vec3::new(-10.0, 4.0, 0.0);
            cam_t.position = (ship_pos + offset).to_array();
            cam_t.orientation = [
                self.player.orientation.x,
                self.player.orientation.y,
                self.player.orientation.z,
                self.player.orientation.w,
            ];
        }

        if let Some(t) = engine.world.get::<Transform>(self.player.entity) {
            let ship_pos = Vec3::from_array(t.position);
            if ship_pos.length() > FLOATING_ORIGIN_THRESHOLD {
                self.shift_world(engine, ship_pos);
            }
        }
    }

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("Planet Game")
        .add_plugin(EditorPlugin::new())
        .run(PlanetGame::default())
}
