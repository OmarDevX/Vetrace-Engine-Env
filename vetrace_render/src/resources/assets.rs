use std::collections::HashMap;

use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

use crate::components::{Material, MaterialHandle, MeshHandle, TextureHandle};
use super::cubemap::CubemapAsset;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MeshVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    /// Dedicated non-overlapping UV set used by baked lightmaps. glTF TEXCOORD_1 maps here.
    #[serde(default)]
    pub lightmap_uv: Vec2,
    pub tangent: Vec4,
    pub color: Vec4,
    #[serde(default)]
    pub joints: [u16; 4],
    #[serde(default)]
    pub weights: [f32; 4],
}

impl Default for MeshVertex {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            lightmap_uv: Vec2::ZERO,
            tangent: Vec4::new(1.0, 0.0, 0.0, 1.0),
            color: Vec4::ONE,
            joints: [0; 4],
            weights: [0.0; 4],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeshAsset {
    pub name: String,
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
    /// Incremented when vertex/index data is replaced in place.
    #[serde(default)]
    pub revision: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TextureAsset {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// RGBA8 pixels in row-major order. The WGPU backend chooses sRGB or
    /// linear sampling per material slot: color/emissive maps use sRGB, while
    /// normal/metallic-roughness/occlusion maps use linear data sampling.
    pub rgba8: Vec<u8>,
    /// Incremented when pixels or dimensions are replaced in place.
    #[serde(default)]
    pub revision: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RenderAssets {
    pub next_mesh: u64,
    pub next_texture: u64,
    #[serde(default)]
    pub next_cubemap: u64,
    pub meshes: HashMap<u64, MeshAsset>,
    pub materials: HashMap<u64, Material>,
    pub textures: HashMap<u64, TextureAsset>,
    #[serde(default)]
    pub cubemaps: HashMap<u64, CubemapAsset>,
    /// Text assets preloaded by the platform adapter. Browser builds use this
    /// map for WGSL/custom post-process sources because WebAssembly cannot read
    /// arbitrary native filesystem paths during rendering.
    #[serde(default)]
    pub text_assets: HashMap<String, String>,
}

impl RenderAssets {
    pub fn insert_mesh(&mut self, mesh: MeshAsset) -> MeshHandle {
        let id = self.next_mesh;
        self.next_mesh = self.next_mesh.saturating_add(1);
        self.meshes.insert(id, mesh);
        MeshHandle(id)
    }

    pub fn insert_material(&mut self, material: Material) -> MaterialHandle {
        let id = self.materials.len() as u64;
        self.materials.insert(id, material);
        MaterialHandle(id)
    }

    pub fn insert_texture(&mut self, texture: TextureAsset) -> TextureHandle {
        let id = self.next_texture;
        self.next_texture = self.next_texture.saturating_add(1);
        self.textures.insert(id, texture);
        TextureHandle(id)
    }

    pub fn set_material(&mut self, handle: MaterialHandle, material: Material) {
        self.materials.insert(handle.0, material);
    }

    pub fn set_mesh(&mut self, handle: MeshHandle, mut mesh: MeshAsset) {
        let revision = self
            .meshes
            .get(&handle.0)
            .map_or(mesh.revision, |current| current.revision.saturating_add(1));
        mesh.revision = mesh.revision.max(revision);
        self.meshes.insert(handle.0, mesh);
    }

    pub fn touch_mesh(&mut self, handle: MeshHandle) -> bool {
        let Some(mesh) = self.meshes.get_mut(&handle.0) else { return false; };
        mesh.revision = mesh.revision.saturating_add(1);
        true
    }

    pub fn set_texture(&mut self, handle: TextureHandle, mut texture: TextureAsset) {
        let revision = self
            .textures
            .get(&handle.0)
            .map_or(texture.revision, |current| current.revision.saturating_add(1));
        texture.revision = texture.revision.max(revision);
        self.textures.insert(handle.0, texture);
    }

    pub fn touch_texture(&mut self, handle: TextureHandle) -> bool {
        let Some(texture) = self.textures.get_mut(&handle.0) else { return false; };
        texture.revision = texture.revision.saturating_add(1);
        true
    }

    /// Inserts a normalized path-addressable text asset, such as WGSL source.
    pub fn insert_text_asset(&mut self, path: impl Into<String>, source: impl Into<String>) {
        let path = path.into();
        self.text_assets.insert(normalize_asset_path(&path), source.into());
    }

    /// Resolves a preloaded text asset using portable forward-slash paths.
    pub fn text_asset(&self, path: &str) -> Option<&str> {
        self.text_assets.get(&normalize_asset_path(path)).map(String::as_str)
    }
}

fn normalize_asset_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches("./").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touching_runtime_assets_advances_revisions() {
        let mut assets = RenderAssets::default();
        let mesh = assets.insert_mesh(MeshAsset::default());
        let texture = assets.insert_texture(TextureAsset {
            name: "test".to_string(),
            width: 1,
            height: 1,
            rgba8: vec![255; 4],
            revision: 0,
        });
        assert!(assets.touch_mesh(mesh));
        assert!(assets.touch_texture(texture));
        assert_eq!(assets.meshes[&mesh.0].revision, 1);
        assert_eq!(assets.textures[&texture.0].revision, 1);
    }

    #[test]
    fn text_assets_use_portable_paths() {
        let mut assets = RenderAssets::default();
        assets.insert_text_asset(r"shaders\water.wgsl", "shader");
        assert_eq!(assets.text_asset("shaders/water.wgsl"), Some("shader"));
    }

    #[test]
    fn replacing_runtime_assets_keeps_revisions_monotonic() {
        let mut assets = RenderAssets::default();
        let mesh = assets.insert_mesh(MeshAsset::default());
        let texture = assets.insert_texture(TextureAsset::default());
        assets.set_mesh(mesh, MeshAsset::default());
        assets.set_texture(texture, TextureAsset::default());
        assert_eq!(assets.meshes[&mesh.0].revision, 1);
        assert_eq!(assets.textures[&texture.0].revision, 1);
    }
}
