use crate::{
    components::components::{Transform, UnreliableSync, Velocity},
    engine::engine::Engine,
    net::{NetPacket, NetServer},
    Behaviour,
};

/// Sends entity transform and velocity updates for all [`UnreliableSync`] entities.
pub struct UnreliableSyncSystem<'a> {
    pub server: &'a mut NetServer,
}

impl<'a> UnreliableSyncSystem<'a> {
    pub fn new(server: &'a mut NetServer) -> Self {
        Self { server }
    }
}

impl<'a> Behaviour for UnreliableSyncSystem<'a> {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        for (entity, _mark, trans) in engine.world.query2::<UnreliableSync, Transform>() {
            let vel = engine
                .world
                .get::<Velocity>(entity)
                .map(|v| v.velocity)
                .unwrap_or([0.0; 3]);
            for addr in self.server.clients.keys() {
                self.server.socket.send_queue.push_back((
                    *addr,
                    NetPacket::TransformSync {
                        entity: entity.0,
                        position: trans.position,
                        orientation: trans.orientation,
                        velocity: vel,
                    },
                ));
            }
        }
        let _ = self.server.socket.flush_send_queue();
    }
}
