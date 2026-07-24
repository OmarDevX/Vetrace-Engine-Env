use super::*;

// Build the active default material fragment module from focused source chunks.
// The legacy material chunks intentionally split a few functions across file
// boundaries, so keep their three-file order contiguous. Environment resource
// declarations are prepended and environment helper functions are appended;
// WGSL permits functions to be defined later than their call sites.
pub(super) const DEFAULT_FRAGMENT_WGSL: &str = concat!(
    include_str!("environment_bindings.wgsl"),
    include_str!("default_material_bindings.wgsl"),
    include_str!("default_material_lighting.wgsl"),
    include_str!("default_material_fragment.wgsl"),
    include_str!("environment_lighting.wgsl"),
);
