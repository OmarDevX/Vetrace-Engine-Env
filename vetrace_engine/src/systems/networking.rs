use crate::components::components::InputBuffer;
use crate::engine::engine::Engine;
use crate::net::{NetPacket, NetServer, NetSyncRegistry};
use crate::Behaviour;

/// Collect input packets from clients and store them in the [`InputBuffer`]
/// for each client-controlled entity.
pub struct ServerInputSystem<'a> {
    pub server: &'a mut NetServer,
}

impl<'a> Behaviour for ServerInputSystem<'a> {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        self.server.poll();
        while let Some((addr, packet)) = self.server.socket.recv_queue.pop_front() {
            if let NetPacket::Input { tick: _, input } = packet {
                if let Some(client_id) = self.server.clients.get(&addr) {
                    if let Some(&entity) = self.server.entity_map.get(client_id) {
                        if let Some((_e, buf)) = engine
                            .world
                            .query_mut::<InputBuffer>()
                            .into_iter()
                            .find(|(e, _)| *e == entity)
                        {
                            buf.inputs.push_back(input);
                        }
                    }
                }
            }
        }
    }
}

/// Sends [`NetPacket::ComponentUpdate`] packets for changed components.
pub struct NetSyncSystem<'a> {
    pub server: &'a mut NetServer,
    pub registry: &'a NetSyncRegistry,
    pub tick_interval: u32,
    tick: u32,
}

impl<'a> NetSyncSystem<'a> {
    pub fn new(server: &'a mut NetServer, registry: &'a NetSyncRegistry, tick_interval: u32) -> Self {
        Self { server, registry, tick_interval, tick: 0 }
    }
}

impl<'a> Behaviour for NetSyncSystem<'a> {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        if self.tick % self.tick_interval == 0 {
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
        }
        self.tick += 1;
    }
}

/// Applies [`NetPacket::ComponentUpdate`] packets received by a client.
pub struct ApplyComponentUpdates<'a> {
    pub client: &'a mut crate::net::NetClient,
    pub registry: &'a NetSyncRegistry,
}

impl<'a> Behaviour for ApplyComponentUpdates<'a> {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        self.client.poll();
        while let Some((_addr, packet)) = self.client.socket.recv_queue.pop_front() {
            if let NetPacket::ComponentUpdate { entity, component, data } = packet {
                let ent = crate::ecs::Entity(entity);
                if let Some(hooks) = self.registry.components.get(component.as_str()) {
                    (hooks.apply)(&mut engine.world, ent, &data);
                }
            }
        }
    }
}
