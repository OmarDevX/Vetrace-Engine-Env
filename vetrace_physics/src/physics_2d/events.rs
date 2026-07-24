use glam::Vec2;
use vetrace_core::Entity;

/// First frame/substep where two accepted colliders overlap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionStarted2D {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub sensor: bool,
    /// Unit normal pointing from A toward B.
    pub normal: Vec2,
    pub point: Vec2,
    pub penetration: f32,
}

/// Emitted after a previously active overlap is no longer present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionStopped2D {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub sensor: bool,
}

/// Current contact information. Emitted for every active pair each physics substep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionContact2D {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub sensor: bool,
    /// Unit normal pointing from A toward B.
    pub normal: Vec2,
    pub point: Vec2,
    pub penetration: f32,
}
