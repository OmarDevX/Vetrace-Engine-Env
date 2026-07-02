use rapier3d::na::{Quaternion, UnitQuaternion, Vector3};
use rapier3d::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use vetrace_engine::components::components;
use vetrace_engine::components::components::{
    ObjectRef, Raycast, RigidBody3D, Transform, UnreliableSync,
};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::Engine;
use vetrace_engine::net::{
    register_sync_component, ClientId, NetPacket, NetServer, NetSyncRegistry,
};
use vetrace_engine::scene::object::Object;
use vetrace_engine::systems::unreliable::UnreliableSyncSystem;
use vetrace_engine::Behaviour;

fn quat_from_yaw(yaw: f32) -> UnitQuaternion<f32> {
    UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw)
}

struct CarState {
    wheel_angle: f32,
    velocity: [f32; 3],
    input: [f32; 2],
    wheel_rays: [Entity; 4],
}

const WHEEL_OFFSETS: [[f32; 3]; 4] = [
    [-0.7, -0.25, 1.2],
    [0.7, -0.25, 1.2],
    [-0.7, -0.25, -1.2],
    [0.7, -0.25, -1.2],
];

impl Default for CarState {
    fn default() -> Self {
        Self {
            wheel_angle: 0.0,
            velocity: [0.0; 3],
            input: [0.0; 2],
            wheel_rays: [Entity(0); 4],
        }
    }
}

struct ServerBehaviour {
    server: NetServer,
    registry: NetSyncRegistry,
    cars: HashMap<u32, (Entity, CarState)>,
    names: HashMap<ClientId, String>,
    chat_log: Vec<String>,
    floor_entity: Entity,
}

impl Behaviour for ServerBehaviour {
    fn start(&mut self, engine: &mut Engine) {
        let mut floor = Object::default();
        floor.position = [0.0, -0.1, 0.0];
        floor.scale = [20.0, 0.2, 20.0];
        floor.is_cube = true;
        engine.spawn_object(floor);
        let id = (engine.scene.objects.len() - 1) as u32;
        if let Some(ent) = engine.core.find_entity_by_object_id(id) {
            engine.world.insert(ent, components::StaticBody::default());
            self.floor_entity = ent;
        }
    }
    fn update(&mut self, engine: &mut Engine, dt: f32) {
        // send component updates from the previous frame
        for (name, hooks) in &self.registry.components {
            let updates = (hooks.collect)(&mut engine.world);
            for (entity, data) in updates {
                for addr in self.server.clients.keys() {
                    self.server.socket.send_queue.push_back((
                        *addr,
                        NetPacket::ComponentUpdate {
                            entity: entity.0,
                            component: (*name).to_string(),
                            data: data.clone(),
                        },
                    ));
                }
            }
        }
        let _ = self.server.socket.flush_send_queue();

        self.server.poll();
        for entity in self.server.disconnected.drain(..) {
            engine.delete_entity(entity);
            self.cars.retain(|_, (e, _)| *e != entity);
            for addr in self.server.clients.keys() {
                self.server
                    .socket
                    .send_queue
                    .push_back((*addr, NetPacket::DespawnObject { entity: entity.0 }));
            }
        }

        // create cars for new clients
        for (&addr, &id) in self.server.clients.iter() {
            if !self.cars.contains_key(&id) {
                let mut obj = Object::default();
                obj.position = [id as f32 * 3.0, 0.5, 0.0];
                obj.scale = [1.5, 0.5, 3.0];
                obj.is_cube = true;
                engine.spawn_object(obj);
                let obj_id = (engine.scene.objects.len() - 1) as u32;
                let entity = engine.core.find_entity_by_object_id(obj_id).unwrap();
                let mut rb = RigidBody3D::default();
                rb.linear_damping = 4.0;
                rb.angular_damping = 4.0;
                engine.world.insert(entity, rb);
                engine.world.insert(entity, UnreliableSync);

                let mut car_state = CarState::default();
                for w in &mut car_state.wheel_rays {
                    let e = engine.world.spawn();
                    let mut ray = Raycast::default();
                    ray.ignore_entity = entity;
                    engine.world.insert(e, ray);
                    *w = e;
                }
                self.cars.insert(id, (entity, car_state));

                self.server
                    .socket
                    .send_queue
                    .push_back((addr, NetPacket::AssignEntity(entity.0)));
                // send floor object to the new client
                if let Some(obj_ref) = engine.world.get::<ObjectRef>(self.floor_entity) {
                    let floor_obj = engine.scene.objects[obj_ref.id as usize].clone();
                    self.server.socket.send_queue.push_back((
                        addr,
                        NetPacket::SpawnObject {
                            entity: self.floor_entity.0,
                            object: floor_obj,
                        },
                    ));
                }
                for (&other_id, (other_ent, _)) in &self.cars {
                    if other_id == id {
                        continue;
                    }
                    let trans = engine.world.get::<Transform>(*other_ent).unwrap();
                    let mut other_obj = Object::default();
                    other_obj.position = trans.position;
                    other_obj.scale = [1.5, 0.5, 3.0];
                    other_obj.is_cube = true;
                    self.server.socket.send_queue.push_back((
                        addr,
                        NetPacket::SpawnObject {
                            entity: other_ent.0,
                            object: other_obj,
                        },
                    ));
                }
                for client in self.server.clients.keys() {
                    self.server.socket.send_queue.push_back((
                        *client,
                        NetPacket::SpawnObject {
                            entity: entity.0,
                            object: obj,
                        },
                    ));
                }
                self.server.entity_map.insert(id, entity);
                let default_name = format!("Player{}", id);
                self.names.entry(id).or_insert(default_name);
                for msg in &self.chat_log {
                    self.server.socket.send_queue.push_back((
                        addr,
                        NetPacket::Custom {
                            kind: "chat".into(),
                            data: bincode::serialize(msg).unwrap(),
                        },
                    ));
                }
            }
        }

        // process input packets
        while let Some((addr, packet)) = self.server.socket.recv_queue.pop_front() {
            match packet {
                NetPacket::Input { input, .. } => {
                    if let Some(client_id) = self.server.clients.get(&addr) {
                        if let Some((_ent, state)) = self.cars.get_mut(client_id) {
                            if let Ok(arr) = bincode::deserialize::<[f32; 2]>(&input.bytes) {
                                state.input = arr;
                            }
                        }
                    }
                }
                NetPacket::Custom { kind, data } => match kind.as_str() {
                    "chat" => {
                        if let Some(client_id) = self.server.clients.get(&addr) {
                            if let Ok(text) = bincode::deserialize::<String>(&data) {
                                let name = self
                                    .names
                                    .get(client_id)
                                    .cloned()
                                    .unwrap_or_else(|| format!("Player{}", client_id));
                                let full = format!("{}: {}", name, text);
                                self.chat_log.push(full.clone());
                                for client_addr in self.server.clients.keys() {
                                    self.server.socket.send_queue.push_back((
                                        *client_addr,
                                        NetPacket::Custom {
                                            kind: "chat".into(),
                                            data: bincode::serialize(&full).unwrap(),
                                        },
                                    ));
                                }
                            }
                        }
                    }
                    "set_name" => {
                        if let Some(client_id) = self.server.clients.get(&addr) {
                            if let Ok(name) = bincode::deserialize::<String>(&data) {
                                self.names.insert(*client_id, name);
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // apply control to cars
        for (_id, (entity, state)) in self.cars.iter_mut() {
            let forward = state.input[0];
            let steer_input = state.input[1];
            let steer_rate = 2.0;
            let wheel_return = 4.0;
            let max_wheel_angle = 0.2;
            let spring_strength = 2.0;
            let spring_damping = 6.0;
            let suspension_rest = 0.5;
            let tire_grip = 0.9;
            let tire_mass = 1.0;
            let car_top_speed = 3.0;
            let engine_power = 8.0;
            let max_speed = 6.0;

            state.wheel_angle += steer_input * steer_rate * dt;
            if steer_input.abs() < f32::EPSILON {
                state.wheel_angle *= 1.0 - wheel_return * dt;
            }
            state.wheel_angle = state.wheel_angle.clamp(-max_wheel_angle, max_wheel_angle);

            if let Some(mut trans) = engine.world.get::<Transform>(*entity).cloned() {
                let q = UnitQuaternion::from_quaternion(Quaternion::new(
                    trans.orientation[3],
                    trans.orientation[0],
                    trans.orientation[1],
                    trans.orientation[2],
                ));
                for (i, ray_e) in state.wheel_rays.iter().enumerate() {
                    if let Some(ray) = engine.world.get_mut::<Raycast>(*ray_e) {
                        let off = Vector3::new(
                            WHEEL_OFFSETS[i][0],
                            WHEEL_OFFSETS[i][1],
                            WHEEL_OFFSETS[i][2],
                        );
                        let world_off = q.transform_vector(&off);
                        let world_pos =
                            Vector3::new(trans.position[0], trans.position[1], trans.position[2])
                                + world_off;
                        ray.origin = [world_pos.x, world_pos.y, world_pos.z];
                        ray.direction = [0.0, -1.0, 0.0];
                        ray.max_distance = suspension_rest + 0.3;
                    }
                }
            }

            if let Some(rb) = engine.world.get_mut::<RigidBody3D>(*entity) {
                if let Some(handle) = rb.handle {
                    if let Some(body) = engine.physics.bodies.get_mut(handle) {
                        let pos = *body.translation();
                        let linvel = *body.linvel();
                        let base_rot = *body.rotation();
                        for (i, ray_e) in state.wheel_rays.iter().enumerate() {
                            if let Some(ray) = engine.world.get::<Raycast>(*ray_e) {
                                if ray.hit_distance < ray.max_distance {
                                    let off = Vector3::new(
                                        WHEEL_OFFSETS[i][0],
                                        WHEEL_OFFSETS[i][1],
                                        WHEEL_OFFSETS[i][2],
                                    );
                                    let wheel_rot = if i < 2 {
                                        base_rot * quat_from_yaw(state.wheel_angle)
                                    } else {
                                        base_rot
                                    };
                                    let world_off = base_rot.transform_vector(&off);
                                    let world_pos = pos + world_off;
                                    let spring_dir = wheel_rot.transform_vector(&Vector3::y_axis());
                                    let steering_dir =
                                        wheel_rot.transform_vector(&Vector3::x_axis());
                                    let accel_dir = wheel_rot.transform_vector(&Vector3::z_axis());
                                    let point = Point::from(world_pos);
                                    let point_vel = body.velocity_at_point(&point);
                                    let offset = (suspension_rest - ray.hit_distance).max(0.0);
                                    if offset > 0.0 {
                                        let vel = spring_dir.dot(&point_vel);
                                        let force = offset * spring_strength - vel * spring_damping;
                                        body.add_force_at_point(spring_dir * force, point, true);
                                    }

                                    let steering_vel = steering_dir.dot(&point_vel);
                                    let desired_vel_change = -steering_vel * tire_grip;
                                    let desired_accel = if dt > 0.0 {
                                        desired_vel_change / dt
                                    } else {
                                        0.0
                                    };
                                    body.add_force_at_point(
                                        steering_dir * tire_mass * desired_accel,
                                        point,
                                        true,
                                    );

                                    if forward.abs() > 0.0 {
                                        let car_forward =
                                            base_rot.transform_vector(&Vector3::z_axis());
                                        let car_speed = car_forward.dot(&linvel);
                                        let normalized_speed =
                                            (car_speed.abs() / car_top_speed).clamp(0.0, 1.0);
                                        let available =
                                            engine_power * (1.0 - normalized_speed) * forward;
                                        body.add_force_at_point(accel_dir * available, point, true);
                                    }
                                }
                            }
                        }

                        // clamp extreme velocities/positions to keep the physics stable
                        let mut lv = *body.linvel();
                        let speed = lv.norm();
                        if speed > max_speed {
                            lv = lv / speed * max_speed;
                            body.set_linvel(lv, true);
                        }
                        let pos = *body.translation();
                        let rot = *body.rotation();
                        let invalid_pos =
                            !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite();
                        let invalid_rot =
                            !rot.i.is_finite() || !rot.j.is_finite() || !rot.k.is_finite() || !rot.w.is_finite();
                        if invalid_pos || pos.norm() > 1000.0 || invalid_rot {
                            body.set_translation(vector![0.0, 2.0, 0.0], true);
                            body.set_linvel(vector![0.0, 0.0, 0.0], true);
                            body.set_angvel(vector![0.0, 0.0, 0.0], true);
                            body.set_rotation(UnitQuaternion::identity(), true);
                        }
                    }
                }
            }
        }

        UnreliableSyncSystem::new(&mut self.server).update(engine, 0.0);

        // capture resulting velocities after physics step of previous frame
        for (_id, (entity, state)) in self.cars.iter_mut() {
            if let Some(rb) = engine.world.get::<RigidBody3D>(*entity) {
                if let Some(handle) = rb.handle {
                    if let Some(body) = engine.physics.bodies.get(handle) {
                        let v = body.linvel();
                        state.velocity = [v.x, v.y, v.z];
                    }
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let addr: SocketAddr = "127.0.0.1:4000".parse().unwrap();
    let server = NetServer::new(addr)?;
    let mut registry = NetSyncRegistry::default();
    register_sync_component::<Transform>(&mut registry);
    let behaviour = ServerBehaviour {
        server,
        registry,
        cars: HashMap::new(),
        names: HashMap::new(),
        chat_log: Vec::new(),
        floor_entity: Entity(0),
    };
    let mut engine = Engine::new(false);
    engine.run_with_behaviour(false, behaviour);
    Ok(())
}
