use std::collections::HashMap;

use vetrace_core::{Actor, ActorId, Engine};

/// Live result of instantiating a scene or prefab fragment.
#[derive(Clone, Debug, Default)]
pub struct SceneInstance {
    pub roots: Vec<Actor>,
    pub actors: Vec<Actor>,
    /// Authoring IDs as written in the scene document.
    pub scene_ids: HashMap<String, Actor>,
    /// Stable runtime-independent IDs attached to the spawned actors.
    pub actor_ids: HashMap<ActorId, Actor>,
}

impl SceneInstance {
    pub fn actor(&self, scene_id: &str) -> Option<Actor> { self.scene_ids.get(scene_id).copied() }
    pub fn actor_by_id(&self, actor_id: ActorId) -> Option<Actor> { self.actor_ids.get(&actor_id).copied() }
    pub fn is_alive(&self, engine: &Engine) -> bool { self.actors.iter().any(|actor| actor.is_alive(engine)) }

    pub fn unload(self, engine: &mut Engine) {
        for root in self.roots { root.despawn(engine); }
    }

    pub(crate) fn record(&mut self, engine: &Engine, scene_id: String, actor: Actor, root: bool) {
        if root { self.roots.push(actor); }
        self.actors.push(actor);
        self.scene_ids.insert(scene_id, actor);
        if let Some(actor_id) = actor.id(engine) {
            self.actor_ids.insert(actor_id, actor);
        }
    }
}
