use serde_json::Value;
use crate::{components::components::{Player, Rotate}, engine::engine::Engine, ecs::Entity};



pub fn rotate_factory(entity: Entity, engine: &mut Engine, data: &Value) {
    let speed = data
        .get("speed")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0) as f32;
    engine.world.insert(entity, Rotate { speed });
}


/// Factory function to create and insert Player component (no extra params needed)
pub fn player_factory(entity: Entity, engine: &mut Engine, _data: &Value) {
    engine.world.insert(entity, Player);
}
