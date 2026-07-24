use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GltfCollisionPolicy {
    /// Only nodes explicitly named as collision helpers are imported as
    /// colliders. Recommended default for authored maps.
    #[default]
    NamedCollisionNodes,
    /// Every imported visible triangle mesh also gets a static triangle-mesh
    /// collider. Useful for quick blocking-out, but can be expensive on high-poly
    /// assets and should not be used for dynamic bodies.
    AutoStaticMesh,
    /// Import authored collision helper nodes and also add static triangle-mesh
    /// colliders to normal visible meshes.
    NamedCollisionNodesAndAutoStaticMesh,
}

impl GltfCollisionPolicy {
    pub fn uses_named_nodes(self) -> bool {
        matches!(self, Self::NamedCollisionNodes | Self::NamedCollisionNodesAndAutoStaticMesh)
    }

    pub fn uses_auto_static_mesh(self) -> bool {
        matches!(self, Self::AutoStaticMesh | Self::NamedCollisionNodesAndAutoStaticMesh)
    }
}

#[derive(Clone, Debug)]
pub struct GltfLoadOptions {
    /// Scene to import. `None` uses the document default scene and falls back to
    /// the first scene if the file has no explicit default.
    pub scene_index: Option<usize>,
    /// Root entity name. `None` uses the glTF scene name or file stem.
    pub root_name: Option<String>,
    /// Import mesh primitives into `RenderAssets` and spawn `Renderable`s.
    pub import_meshes: bool,
    /// Import glTF PBR material factors into renderer `Material`s.
    pub import_materials: bool,
    /// Import decoded glTF image data used by material base-color textures.
    pub import_textures: bool,
    /// Build smooth normals when a primitive has positions/indices but no normals.
    pub generate_missing_normals: bool,
    /// Import KHR_lights_punctual lights attached to glTF nodes.
    pub import_lights: bool,
    /// Import collision metadata from glTF names into renderer-neutral
    /// `GltfImportedCollider` components. Runtime Rapier creation is handled by
    /// `vetrace_physics` when built with its `gltf_collisions` feature.
    pub import_collisions: bool,
    /// Controls which meshes become imported collider hints.
    pub collision_policy: GltfCollisionPolicy,
    /// Import glTF animation clips and attach an AnimationPlayer to the imported root.
    #[cfg(feature = "gltf_animation")]
    pub import_animations: bool,
}

impl GltfLoadOptions {
    /// Convenience preset for quick map import: render the scene and generate a
    /// static triangle-mesh collider for each visible mesh, while still honoring
    /// authored `COL_`/`TRIGGER_` helper nodes.
    pub fn static_map() -> Self {
        Self {
            import_collisions: true,
            collision_policy: GltfCollisionPolicy::NamedCollisionNodesAndAutoStaticMesh,
            ..Self::default()
        }
    }

    /// Convenience preset for production-authored glTFs where collision helpers
    /// are explicitly named and visual meshes should not automatically collide.
    pub fn authored_collision_nodes() -> Self {
        Self {
            import_collisions: true,
            collision_policy: GltfCollisionPolicy::NamedCollisionNodes,
            ..Self::default()
        }
    }
}

impl Default for GltfLoadOptions {
    fn default() -> Self {
        Self {
            scene_index: None,
            root_name: None,
            import_meshes: true,
            import_materials: true,
            import_textures: true,
            generate_missing_normals: true,
            import_lights: true,
            // Cheap/safe by default: only explicit collision helper nodes produce
            // collider hints. Normal visual meshes stay visual-only unless the
            // caller asks for `static_map()` or `AutoStaticMesh`.
            import_collisions: true,
            collision_policy: GltfCollisionPolicy::NamedCollisionNodes,
            #[cfg(feature = "gltf_animation")]
            import_animations: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GltfLoadReport {
    pub root: Entity,
    pub path: PathBuf,
    pub nodes_spawned: usize,
    pub mesh_primitives_loaded: usize,
    pub materials_loaded: usize,
    pub textures_loaded: usize,
    pub lights_loaded: usize,
    pub collision_nodes_loaded: usize,
    pub auto_colliders_loaded: usize,
    pub skipped_primitives: usize,
    #[cfg(feature = "gltf_animation")]
    pub animations_loaded: usize,
    #[cfg(feature = "gltf_animation")]
    pub animation_channels_loaded: usize,
}
