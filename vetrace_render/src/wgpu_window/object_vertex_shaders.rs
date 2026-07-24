use super::*;

// The camera/object ABI is defined once and shared by every vertex interface.
// This prevents the full, textured, and legacy variants from drifting apart.
pub(super) const OBJECT_VERTEX_WGSL: &str = concat!(
    include_str!("object_vertex_bindings.wgsl"),
    include_str!("object_vertex_full.wgsl"),
);

pub(super) const TEXTURED_OBJECT_VERTEX_WGSL: &str = concat!(
    include_str!("object_vertex_bindings.wgsl"),
    include_str!("object_vertex_textured.wgsl"),
);

pub(super) const LEGACY_OBJECT_VERTEX_WGSL: &str = concat!(
    include_str!("object_vertex_bindings.wgsl"),
    include_str!("object_vertex_legacy.wgsl"),
);
