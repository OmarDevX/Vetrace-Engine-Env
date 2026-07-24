use super::*;

/// Renderer-neutral glTF collision import metadata.
///
/// The glTF loader creates this component when collision import is enabled. It
/// deliberately does **not** create Rapier colliders. `vetrace_physics` can opt
/// into consuming this data through its `gltf_collisions` feature, keeping the
/// render/import layer and runtime physics layer separated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GltfCollisionBodyKind {
    #[default]
    Static,
    Dynamic,
    Kinematic,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GltfCollisionShapeKind {
    #[default]
    Box,
    Sphere,
    Capsule,
    ConvexHull,
    TriangleMesh,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GltfImportedCollider {
    pub body: GltfCollisionBodyKind,
    pub shape: GltfCollisionShapeKind,
    /// Whether this collider is a trigger/sensor instead of a solid collider.
    pub sensor: bool,
    /// Local-space half extents for primitive shapes. Also acts as a primitive
    /// fallback if a named collision node has no mesh data.
    pub half_extents: Vec3,
    /// Local-space offset from the node origin. For primitive colliders this is
    /// usually the mesh AABB center.
    pub offset: Vec3,
    /// Local-space vertices used by convex hull and triangle mesh collision.
    pub vertices: Vec<Vec3>,
    /// Triangle indices used by triangle mesh collision. Convex hull import does
    /// not need them, but keeping them allows tools to switch shape kind later.
    pub indices: Vec<[u32; 3]>,
    /// Human-readable source hint for debugging/import reports.
    pub source: String,
}

impl Default for GltfImportedCollider {
    fn default() -> Self {
        Self {
            body: GltfCollisionBodyKind::Static,
            shape: GltfCollisionShapeKind::Box,
            sensor: false,
            half_extents: Vec3::splat(0.5),
            offset: Vec3::ZERO,
            vertices: Vec::new(),
            indices: Vec::new(),
            source: String::new(),
        }
    }
}
