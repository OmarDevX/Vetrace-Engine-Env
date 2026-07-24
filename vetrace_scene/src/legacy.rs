use serde::Deserialize;
use vetrace_physics::PhysicsColliderDef;
use vetrace_primitives::PrimitiveKind;

use crate::{SceneMaterial, SceneTransform};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LegacyPrefabDocument {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub objects: Vec<LegacyPrefabObject>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LegacyPrefabObject {
    pub id: String,
    pub name: String,
    pub primitive: PrimitiveKind,
    pub transform: SceneTransform,
    pub size: [f32; 3],
    #[serde(default)]
    pub material: SceneMaterial,
    pub collider: Option<PhysicsColliderDef>,
    #[serde(default)]
    pub tags: Vec<String>,
}
