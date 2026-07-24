//! Explicit baked diffuse lighting API.
//!
//! Normal runtime only loads `.vlight` files. Baking is started deliberately
//! through [`bake_and_save_baked_lighting`].

use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::Path;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use vetrace_core::components::builtins::{GlobalTransform, Transform};
use vetrace_core::engine::Engine;

use crate::backend::RenderObject;
use crate::components::{
    AlphaMode, BakedLightProbeDebugMarker, BakedLightProbeReceiver, Material,
    PrimitiveShape, Renderable, Shape,
};
use crate::resources::{
    BakedLightingDebugMode, BakedLightingFile, BakedLightingRuntimeMode,
    BakedLightingScene, BakedLightmapAtlas, BakedLightmapRegion, BakedProbeSample,
    RenderAssets, BAKED_LIGHTING_FILE_VERSION,
};

use crate::baked_lighting_bake::bake_baked_lighting;
pub(crate) use crate::baked_lighting_geometry::cube_face_lightmap_uv;

mod config;
mod debug;
mod file_io;
mod object_key;
mod runtime_bindings;

pub use config::{BakedLightingBakeConfig, BakedLightingBakeReport};
pub use debug::{
    baked_lighting_debug_mode, baked_lighting_runtime_mode,
    cycle_baked_lighting_debug_mode, set_baked_lighting_debug_mode,
    set_baked_lighting_runtime_mode, unload_baked_lighting,
};
pub use file_io::{bake_and_save_baked_lighting, load_baked_lighting};
#[cfg(test)]
use file_io::validate_file;
pub(crate) use object_key::baked_object_key;
pub(crate) use runtime_bindings::{
    render_baked_lighting_for_object, RenderBakedLightmap, RenderBakedProbes,
};

#[cfg(test)]
#[path = "baked_lighting/tests.rs"]
mod tests;
