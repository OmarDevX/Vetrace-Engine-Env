use super::*;

/// Opaque mesh asset handle. Real renderer backends can map this id to GPU data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeshHandle(pub u64);

/// Opaque material asset handle. Real renderer backends can map this id to GPU data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialHandle(pub u64);

/// Opaque texture asset handle. Real renderer backends can map this id to GPU texture data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextureHandle(pub u64);

/// Opaque cubemap asset handle used by global environments and reflection probes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CubemapHandle(pub u64);

/// Simple renderable marker tying an entity to mesh/material handles.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Renderable {
    pub mesh: Option<MeshHandle>,
    pub material: Option<MaterialHandle>,
    pub visible: bool,
}

/// Imported object mesh metadata preserved from the old monolithic component set.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ObjMesh {
    pub mesh: MeshHandle,
    pub material: Option<MaterialHandle>,
    pub submesh_entities: Vec<vetrace_core::Entity>,
}
