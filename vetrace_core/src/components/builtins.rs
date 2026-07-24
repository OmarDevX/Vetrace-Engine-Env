use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generic transform component used by most games and plugins.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}


/// Stable identity used by scenes, saves, networking, and editor references.
///
/// Unlike [`crate::Entity`], this value survives runtime slot reuse and may be
/// serialized safely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActorId(pub Uuid);

impl ActorId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
    pub const fn from_uuid(id: Uuid) -> Self { Self(id) }
    pub const fn uuid(self) -> Uuid { self.0 }
}

impl Default for ActorId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for ActorId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Marks a local transform whose derived world transform needs refreshing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransformDirty;

/// Generic human-readable entity label.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Name(pub String);

/// World-space transform produced by hierarchy/transform propagation systems.
///
/// The core crate owns this because render, physics, audio, scripting, and user
/// gameplay plugins may all need world-space entity transforms without depending
/// on each other.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for GlobalTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl From<Transform> for GlobalTransform {
    fn from(transform: Transform) -> Self {
        Self {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
}

impl From<&Transform> for GlobalTransform {
    fn from(transform: &Transform) -> Self {
        Self {
            translation: transform.translation,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
}

/// Parent relation for scene hierarchy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Parent(pub crate::ecs::Entity);

/// Compatibility mirror of hierarchy children. `Parent` is the single source
/// of truth; new code should use `Actor::children` instead of mutating this.
#[deprecated(note = "Parent is authoritative; use Actor hierarchy methods")]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Children(pub Vec<crate::ecs::Entity>);

/// Stable id/reference component for imported scenes, editor selections, and
/// runtime lookups. This replaces old ad-hoc object reference components.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectRef {
    pub id: u64,
}

/// Generic metadata component. Keep arbitrary tags here instead of adding dirty
/// one-off game-specific tags.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub tags: Vec<String>,
    pub source: Option<String>,
}

/// General-purpose timer utility. Kept in core because it is not tied to a
/// renderer, physics engine, audio engine, or scripting language.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Timer {
    pub duration: f32,
    pub elapsed: f32,
    pub repeating: bool,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            duration: 1.0,
            elapsed: 0.0,
            repeating: false,
        }
    }
}

impl Timer {
    pub fn new(duration: f32) -> Self {
        Self { duration, ..Self::default() }
    }

    pub fn repeating(duration: f32) -> Self {
        Self { duration, repeating: true, ..Self::default() }
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            if self.repeating && self.duration > 0.0 {
                self.elapsed %= self.duration;
            }
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.elapsed = 0.0;
    }
}
