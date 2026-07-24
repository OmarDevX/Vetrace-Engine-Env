pub const SCENE_VERSION: u32 = 1;

/// Compatibility name for older code and files. A prefab is a scene fragment.
pub const PREFAB_VERSION: u32 = SCENE_VERSION;

/// Namespaced built-in scene component IDs. The scene format is intentionally
/// open: subsystem crates can add new component IDs without changing the core
/// scene document shape.
pub mod component_type {
    pub const PRIMITIVE: &str = "vetrace.render.primitive";
    pub const MATERIAL: &str = "vetrace.render.material";
    pub const PHYSICS_BODY: &str = vetrace_physics::SCENE_PHYSICS_BODY_COMPONENT;
    pub const PHYSICS_COLLIDER: &str = vetrace_physics::SCENE_PHYSICS_COLLIDER_COMPONENT;
    pub const SPAWN_POINT: &str = "vetrace.scene.spawn_point";
    pub const WORLD_LABEL: &str = "vetrace.ui.world_label";
    pub const AUDIO_SOURCE: &str = "vetrace.audio.source";
    pub const PREFAB_INSTANCE: &str = "vetrace.scene.prefab_instance";
    pub const TAGS: &str = "vetrace.scene.tags";
}

pub(crate) const PRIMITIVE_ALIASES: &[&str] = &[component_type::PRIMITIVE, "primitive"];
pub(crate) const MATERIAL_ALIASES: &[&str] = &[component_type::MATERIAL, "material"];
pub(crate) const PHYSICS_BODY_ALIASES: &[&str] = &[component_type::PHYSICS_BODY, "physics_body", "rigid_body"];
pub(crate) const PHYSICS_COLLIDER_ALIASES: &[&str] = &[component_type::PHYSICS_COLLIDER, "physics_collider", "collider"];
pub(crate) const TAGS_ALIASES: &[&str] = &[component_type::TAGS, "tags"];
