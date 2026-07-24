//! CPU lightmap/probe baker. This module is never entered by normal loading.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::f32::consts::PI;

use glam::{Vec2, Vec3, Vec4};
use vetrace_core::components::builtins::{GlobalTransform, Transform};
use vetrace_core::engine::Engine;

use crate::backend::{
    build_render_frame, RenderDirectionalLight, RenderPointLight, RenderSpotLight,
};
use crate::baked_lighting::{
    baked_object_key, BakedLightingBakeConfig, BakedLightingBakeReport,
};
use crate::baked_lighting_geometry::{
    append_object_triangles, barycentric_2d, trace_any, trace_nearest, triangle_bounds,
    BakeTriangle,
};
use crate::components::{BakedLightmapReceiver, BakedRectAreaLight};
use crate::resources::{
    BakedLightingFile, BakedLightmapRegion, BakedProbeGrid, BakedProbeSample,
    RenderAssets, BAKED_LIGHTING_FILE_VERSION,
};

mod atlas;
mod configuration;
mod lighting;
mod lightmaps;
mod orchestrator;
mod probes;
mod sampling;
mod types;

pub(crate) use orchestrator::bake_baked_lighting;
use atlas::*;
use configuration::*;
use lighting::*;
use lightmaps::*;
use probes::*;
use sampling::*;
use types::*;

#[cfg(test)]
#[path = "baked_lighting_bake_tests.rs"]
mod tests;
