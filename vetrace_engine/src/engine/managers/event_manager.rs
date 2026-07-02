use crate::ecs::Entity;
use crate::events::{Event as CustomEvent, SceneEvents};
use crate::systems::collision::CollisionEvent;

/// Manages events and engine state
pub struct EventManager {
    pub collision_events: Vec<CollisionEvent>,
    pub collision_event: CustomEvent<CollisionEvent>,
    pub entity_events: Vec<(Entity, Entity, String)>,
    pub entity_event: CustomEvent<(Entity, Entity, String)>,
    pub scene_events: SceneEvents,
    pub running: bool,
    pub paused: bool,
    pub is_2d: bool,
}

impl EventManager {
    pub fn new(is_2d: bool) -> Self {
        Self {
            collision_events: Vec::new(),
            collision_event: CustomEvent::new(),
            entity_events: Vec::new(),
            entity_event: CustomEvent::new(),
            scene_events: SceneEvents::new(),
            running: true,
            paused: false,
            is_2d,
        }
    }

    /// Add a collision event
    pub fn add_collision_event(&mut self, event: CollisionEvent) {
        self.collision_events.push(event);
    }

    /// Add an entity event
    pub fn add_entity_event(&mut self, entity1: Entity, entity2: Entity, event_type: String) {
        self.entity_events.push((entity1, entity2, event_type));
    }

    /// Clear collision events
    pub fn clear_collision_events(&mut self) {
        self.collision_events.clear();
    }

    /// Clear entity events
    pub fn clear_entity_events(&mut self) {
        self.entity_events.clear();
    }

    /// Check if engine is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Set engine running state
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// Check if engine is paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Set engine paused state
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Check if engine is in 2D mode
    pub fn is_2d(&self) -> bool {
        self.is_2d
    }

    /// Set 2D mode
    pub fn set_2d(&mut self, is_2d: bool) {
        self.is_2d = is_2d;
    }

    /// Get collision events reference
    pub fn collision_events(&self) -> &Vec<CollisionEvent> {
        &self.collision_events
    }

    /// Get entity events reference
    pub fn entity_events(&self) -> &Vec<(Entity, Entity, String)> {
        &self.entity_events
    }

    /// Get scene events reference
    pub fn scene_events(&self) -> &SceneEvents {
        &self.scene_events
    }

    /// Get mutable scene events reference
    pub fn scene_events_mut(&mut self) -> &mut SceneEvents {
        &mut self.scene_events
    }
}
