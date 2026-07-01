use std::collections::HashMap;
use std::net::SocketAddr;

use sdl2::keyboard::Keycode;
use vetrace_engine::components::components::{
    Anchor, Transform, UILabel, UILayout, UIList, UIScreenSpace, UITextEditor, Velocity,
};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::Engine;
use vetrace_engine::net::{
    register_sync_component, InputData, NetClient, NetPacket, NetSyncRegistry,
};
use vetrace_engine::scene::object::Object;
use vetrace_engine::Behaviour;

const WHEEL_OFFSETS: [[f32; 3]; 4] = [
    [-0.7, -0.25, 1.2],
    [0.7, -0.25, 1.2],
    [-0.7, -0.25, -1.2],
    [0.7, -0.25, -1.2],
];

struct ClientBehaviour {
    client: NetClient,
    registry: NetSyncRegistry,
    entities: HashMap<u32, Entity>,
    player_id: Option<u32>,
    chat_messages: Vec<String>,
    chat_list: Option<Entity>,
    chat_input: Option<Entity>,
    name_input: Option<Entity>,
    help_label: Option<Entity>,
    name: String,
}

impl Behaviour for ClientBehaviour {
    fn start(&mut self, engine: &mut Engine) {
        self.client.send_ping();

        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UIList>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::BottomLeft;
                layout.offset = [10.0, -220.0];
            }
            if let Some(list) = actor.get_component_mut::<UIList>() {
                list.size = [400.0, 160.0];
                list.color = [0.0, 0.0, 0.0, 255.0];
            }
            self.chat_list = Some(actor.entity());
        }

        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UITextEditor>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::BottomLeft;
                layout.offset = [10.0, -50.0];
            }
            self.chat_input = Some(actor.entity());
        }

        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UITextEditor>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::BottomLeft;
                layout.offset = [420.0, -50.0];
            }
            self.name_input = Some(actor.entity());
        }

        // instructions label
        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UILabel>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::BottomLeft;
                layout.offset = [10.0, -30.0];
            }
            if let Some(label) = actor.get_component_mut::<UILabel>() {
                label.text = "Press Enter to send chat. F2 changes name".into();
                label.font_size = 14.0;
                label.color = [0.0, 0.0, 0.0, 255.0];
            }
            self.help_label = Some(actor.entity());
        }

        self.chat_messages
            .push("Connected. Press Enter to chat.".to_string());

        self.client.socket.send_queue.push_back((
            self.client.server_addr,
            NetPacket::Custom {
                kind: "set_name".into(),
                data: bincode::serialize(&self.name).unwrap(),
            },
        ));
        let _ = self.client.socket.flush_send_queue();
    }

    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        if engine.input.was_key_pressed(Keycode::Return) {
            if let Some(ent) = self.chat_input {
                if let Some(editor) = engine.world.get_mut::<UITextEditor>(ent) {
                    let msg = editor.text.trim().to_string();
                    if !msg.is_empty() {
                        self.client.socket.send_queue.push_back((
                            self.client.server_addr,
                            NetPacket::Custom {
                                kind: "chat".into(),
                                data: bincode::serialize(&msg).unwrap(),
                            },
                        ));
                        editor.text.clear();
                    }
                }
            }
        }

        if engine.input.was_key_pressed(Keycode::F2) {
            if let Some(ent) = self.name_input {
                if let Some(editor) = engine.world.get_mut::<UITextEditor>(ent) {
                    let new_name = editor.text.trim().to_string();
                    if !new_name.is_empty() && new_name != self.name {
                        self.name = new_name.clone();
                        self.client.socket.send_queue.push_back((
                            self.client.server_addr,
                            NetPacket::Custom {
                                kind: "set_name".into(),
                                data: bincode::serialize(&new_name).unwrap(),
                            },
                        ));
                    }
                }
            }
        }

        let mut dir = [0.0f32, 0.0f32];
        if engine.input.is_key_down(Keycode::W) {
            dir[1] += 1.0;
        }
        if engine.input.is_key_down(Keycode::S) {
            dir[1] -= 1.0;
        }
        if engine.input.is_key_down(Keycode::D) {
            dir[0] += 1.0;
        }
        if engine.input.is_key_down(Keycode::A) {
            dir[0] -= 1.0;
        }
        if dir != [0.0, 0.0] {
            let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
            if len > 0.0 {
                dir[0] /= len;
                dir[1] /= len;
            }
        }
        let bytes = bincode::serialize(&dir).unwrap();
        self.client.socket.send_queue.push_back((
            self.client.server_addr,
            NetPacket::Input {
                tick: 0,
                input: InputData { bytes },
            },
        ));
        let _ = self.client.socket.flush_send_queue();

        self.client.poll();
        while let Some((_addr, packet)) = self.client.socket.recv_queue.pop_front() {
            match packet {
                NetPacket::AssignEntity(id) => {
                    self.player_id = Some(id);
                }
                NetPacket::SpawnObject { entity, object } => {
                    self.entities
                        .entry(entity)
                        .or_insert_with(|| engine.spawn_object_as_actor(object).unwrap().entity());
                }
                NetPacket::DespawnObject { entity } => {
                    if let Some(local) = self.entities.remove(&entity) {
                        engine.delete_entity(local);
                    }
                }
                NetPacket::ComponentUpdate {
                    entity,
                    component,
                    data,
                } => {
                    let local = *self.entities.entry(entity).or_insert_with(|| {
                        engine
                            .spawn_object_as_actor(Object::default())
                            .unwrap()
                            .entity()
                    });
                    if let Some(hooks) = self.registry.components.get(component.as_str()) {
                        (hooks.apply)(&mut engine.world, local, &data);
                        if Some(entity) == self.player_id {
                            if let Some((_e, _t)) = engine
                                .world
                                .query::<Transform>()
                                .into_iter()
                                .find(|(e, _)| *e == local)
                            {
                                // player position available here
                            }
                        }
                    }
                }
                NetPacket::TransformSync {
                    entity,
                    position,
                    orientation,
                    velocity,
                } => {
                    let local = *self.entities.entry(entity).or_insert_with(|| {
                        engine
                            .spawn_object_as_actor(Object::default())
                            .unwrap()
                            .entity()
                    });
                    if let Some(mut t) = engine.world.get_mut::<Transform>(local) {
                        t.position = position;
                        t.orientation = orientation;
                    }
                    if let Some(mut v) = engine.world.get_mut::<Velocity>(local) {
                        v.velocity = velocity;
                    }
                }
                NetPacket::Custom { kind, data } => {
                    if kind == "chat" {
                        if let Ok(text) = bincode::deserialize::<String>(&data) {
                            self.chat_messages.push(text);
                            if self.chat_messages.len() > 50 {
                                self.chat_messages.remove(0);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(list_ent) = self.chat_list {
            if let Some(list) = engine.world.get_mut::<UIList>(list_ent) {
                list.items = self.chat_messages.clone();
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let server_addr: SocketAddr = "127.0.0.1:4000".parse().unwrap();
    let client = NetClient::connect(server_addr)?;
    let mut registry = NetSyncRegistry::default();
    register_sync_component::<Transform>(&mut registry);
    register_sync_component::<Velocity>(&mut registry);
    let behaviour = ClientBehaviour {
        client,
        registry,
        entities: HashMap::new(),
        player_id: None,
        chat_messages: Vec::new(),
        chat_list: None,
        chat_input: None,
        name_input: None,
        help_label: None,
        name: "Player".into(),
    };
    let mut engine = Engine::new(true);
    engine.run_with_behaviour(false, behaviour);
    Ok(())
}
