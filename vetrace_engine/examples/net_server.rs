use std::collections::HashMap;
use std::net::SocketAddr;
use vetrace_engine::components::components;
use rapier3d::prelude::*;
use rapier3d::na::{UnitQuaternion, Vector3};
use vetrace_engine::components::components::{ObjectRef, RigidBody3D, Transform};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::Engine;
use vetrace_engine::net::{
    register_sync_component, NetPacket, NetServer, NetSyncRegistry, ClientId,
};
use vetrace_engine::scene::object::Object;
use vetrace_engine::Behaviour;

fn quat_from_yaw(yaw: f32) -> UnitQuaternion<f32> {
    UnitQuaternion::from_axis_angle(&Vector3::y_axis(), yaw)
}

#[derive(Default)]
struct CarState {
    yaw: f32,
    wheel_angle: f32,
    velocity: [f32; 3],
    height_vel: f32,
    input: [f32; 2],
}

struct ServerBehaviour {
    server: NetServer,
    registry: NetSyncRegistry,
    cars: HashMap<u32, (Entity, CarState)>,
    names: HashMap<ClientId, String>,
    chat_log: Vec<String>,
}

impl Behaviour for ServerBehaviour {
    fn update(&mut self, engine: &mut Engine, dt: f32) {
        // send component updates from the previous frame
        for (name, hooks) in &self.registry.components {
            let updates = (hooks.collect)(&mut engine.world);
            for (entity, data) in updates {
                for addr in self.server.clients.keys() {
                    self.server.socket.send_queue.push_back((*addr, NetPacket::ComponentUpdate { entity: entity.0, component: (*name).to_string(), data: data.clone() }));
                }
            }
        }
        let _ = self.server.socket.flush_send_queue();

        self.server.poll();
        for entity in self.server.disconnected.drain(..) {
            engine.delete_entity(entity);
            self.cars.retain(|_, (e, _)| *e != entity);
            for addr in self.server.clients.keys() {
                self.server.socket.send_queue.push_back((*addr, NetPacket::DespawnObject { entity: entity.0 }));
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
                rb.gravity_enabled = false;
                rb.linear_damping = 4.0;
                rb.angular_damping = 4.0;
                engine.world.insert(entity, rb);
                self.cars.insert(id, (entity, CarState::default()));

                self.server.socket.send_queue.push_back((addr, NetPacket::AssignEntity(entity.0)));
                for (&other_id, (other_ent, _)) in &self.cars {
                    if other_id == id { continue; }
                    let trans = engine.world.get::<Transform>(*other_ent).unwrap();
                    let mut other_obj = Object::default();
                    other_obj.position = trans.position;
                    other_obj.scale = [1.5, 0.5, 3.0];
                    other_obj.is_cube = true;
                    self.server.socket.send_queue.push_back((addr, NetPacket::SpawnObject { entity: other_ent.0, object: other_obj }));
                }
                for client in self.server.clients.keys() {
                    self.server.socket.send_queue.push_back((*client, NetPacket::SpawnObject { entity: entity.0, object: obj }));
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
                NetPacket::Custom { kind, data } => {
                    match kind.as_str() {
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
                    }
                }
                _ => {}
            }
        }

        // apply control to cars
        for (_id, (entity, state)) in self.cars.iter_mut() {
            let forward = state.input[0];
            let steer_input = state.input[1];
            let forward_acc = 80.0;
            let steer_rate = 2.0;
            let wheel_return = 4.0;
            let max_wheel_angle = 0.2;
            let turn_factor = 2.0;
            let damping = 0.9;
            let spring_strength = 10.0;
            let spring_damping = 0.8;
            let target_height = 0.5;

            state.wheel_angle += steer_input * steer_rate * dt;
            if steer_input.abs() < f32::EPSILON {
                state.wheel_angle *= 1.0 - wheel_return * dt;
            }
            state.wheel_angle = state.wheel_angle.clamp(-max_wheel_angle, max_wheel_angle);

            let speed = (state.velocity[0] * state.velocity[0] + state.velocity[2] * state.velocity[2]).sqrt();
            let yaw_delta = state.wheel_angle * speed * turn_factor * dt;
            if yaw_delta.abs() > 0.0 {
                let cosd = yaw_delta.cos();
                let sind = yaw_delta.sin();
                let vx = state.velocity[0] * cosd + state.velocity[2] * sind;
                let vz = -state.velocity[0] * sind + state.velocity[2] * cosd;
                state.velocity[0] = vx;
                state.velocity[2] = vz;
            }

            state.yaw += yaw_delta;
            let dir = [state.yaw.sin(), 0.0, state.yaw.cos()];
            state.velocity[0] += dir[0] * forward * forward_acc * dt;
            state.velocity[2] += dir[2] * forward * forward_acc * dt;
            state.velocity[0] *= damping;
            state.velocity[2] *= damping;

            let y = engine.world.get::<Transform>(*entity).map(|t| t.position[1]).unwrap_or(target_height);
            let diff = target_height - y;
            state.height_vel += diff * spring_strength * dt;
            state.height_vel *= spring_damping;

            if let Some(rb) = engine.world.get_mut::<RigidBody3D>(*entity) {
                if let Some(handle) = rb.handle {
                    if let Some(body) = engine.physics.bodies.get_mut(handle) {
                        body.set_linvel(vector![state.velocity[0], state.height_vel, state.velocity[2]], true);
                        body.set_rotation(quat_from_yaw(state.yaw), true);
                    }
                }
            }

            if let Some(t) = engine.world.get_mut::<Transform>(*entity) {
                let q = quat_from_yaw(state.yaw);
                t.orientation = [q.coords.x, q.coords.y, q.coords.z, q.coords.w];
            }

            if let Some(obj_ref) = engine.world.get::<crate::components::ObjectRef>(*entity) {
                if let Some(obj) = engine.scene.objects.get_mut(obj_ref.id as usize) {
                    let q = quat_from_yaw(state.yaw);
                    obj.orientation = [q.coords.x, q.coords.y, q.coords.z, q.coords.w];
                }
            }
        }

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
    };
    let mut engine = Engine::new(false);
    engine.run_with_behaviour(false, behaviour);
    Ok(())
}
