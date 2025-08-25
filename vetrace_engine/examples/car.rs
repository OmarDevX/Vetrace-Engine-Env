use sdl2::keyboard::Keycode;
use vetrace_engine::components::components::{
    AngularVelocity, Collider, RevoluteJoint, RigidBody3D, StaticBody,
};
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;
use vetrace_engine::Behaviour;

struct CarController {
    body: u32,
    wheels: Vec<u32>,
}

impl Behaviour for CarController {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        let mut torque = 0.0f32;
        if engine.input.is_key_down(Keycode::W) {
            torque = 1.0;
        }
        if engine.input.is_key_down(Keycode::S) {
            torque = -1.0;
        }
        for id in &self.wheels {
            if let Some(ent) = engine.core.find_entity_by_object_id(*id) {
                if let Some(av) = engine.get_component_mut_entity::<AngularVelocity>(ent) {
                    av.angular_acceleration = [torque, 0.0, 0.0];
                    if torque.abs() < f32::EPSILON {
                        // prevent residual wheel spin when no input is given
                        av.angular_velocity = [0.0, 0.0, 0.0];
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
    floor.position = [0.0, -2.0, 0.0];
    floor.scale = [20.0, 1.0, 20.0];
    floor.is_cube = true;
    engine.spawn_object(floor);
    let floor_id = (engine.scene.objects.len() - 1) as u32;
    if let Some(ent) = engine.core.find_entity_by_object_id(floor_id) {
        engine.world.insert(ent, StaticBody::default());
    }

    // car body
    let mut body = Object::default();
    body.position = [0.0, 0.5, 0.0];
    body.scale = [2.0, 0.5, 1.0];
    body.is_cube = true;
    engine.spawn_object(body);
    let body_id = (engine.scene.objects.len() - 1) as u32;
    if let Some(ent) = engine.core.find_entity_by_object_id(body_id) {
        let mut rb = RigidBody3D::default();
        rb.mass = 5.0;
        rb.friction = 1.0;
        rb.linear_damping = 0.5;
        rb.angular_damping = 0.5;
        engine.world.insert(ent, rb);
    }

    // wheels
    let offsets = [
        [-0.7, -0.2, 0.5],
        [0.7, -0.2, 0.5],
        [-0.7, -0.2, -0.5],
        [0.7, -0.2, -0.5],
    ];
    let mut wheel_ids = Vec::new();
    for off in offsets {
        let mut wheel = Object::default();
        wheel.position = [
            body.position[0] + off[0],
            body.position[1] + off[1],
            body.position[2] + off[2],
        ];
        wheel.radius = 0.25;
        wheel.is_cube = false;
        // Use low smoothness for wheels to avoid huge vertex buffers
        engine.spawn_object(wheel);
        let wid = (engine.scene.objects.len() - 1) as u32;
        wheel_ids.push(wid);
        if let Some(ent) = engine.core.find_entity_by_object_id(wid) {
            let mut rb = RigidBody3D::default();
            rb.gravity_enabled = false;
            rb.mass = 0.5;
            rb.friction = 1.0;
            rb.linear_damping = 0.5;
            rb.angular_damping = 0.5;
            engine.world.insert(ent, rb);
            engine.world.insert(
                ent,
                RevoluteJoint {
                    other: body_id,
                    axis: [1.0, 0.0, 0.0],
                    contacts_enabled: false,
                    handle: None,
                },
            );
            if let Some(col) = engine.world.get_mut::<Collider>(ent) {
                col.radius = wheel.radius;
            }
            engine.world.insert(ent, AngularVelocity::default());
        }
    }

    engine.add_behaviour(CarController {
        body: body_id,
        wheels: wheel_ids,
    });

    engine.run(true);
}
