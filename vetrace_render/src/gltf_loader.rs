//! Optional glTF/GLB import for the active render crate.
//!
//! This module is intentionally behind the `gltf_loader` feature.  It parses
//! glTF into renderer-owned mesh/material assets, then spawns normal core ECS
//! entities with `Transform`, `Parent`, and `Renderable` components.
//! It may also attach renderer-neutral `GltfImportedCollider` hints for authored
//! collision nodes, but it does not create Rapier bodies/colliders directly.

use std::path::{Path, PathBuf};
#[cfg(feature = "gltf_animation")]
use std::collections::HashMap;

use anyhow::{Context, Result};
use glam::{Quat, Vec2, Vec3, Vec4};
use vetrace_core::{propagate_global_transforms, Actor, Engine, Entity, Transform};

use crate::components::{
    AlphaMode, DirectionalLight, GltfCollisionBodyKind, GltfCollisionShapeKind, GltfImportedCollider,
    Material, MaterialHandle, ObjMesh, PointLight, Renderable, SpotLight, TextureHandle,
};
use crate::resources::{MeshAsset, MeshVertex, RenderAssets, TextureAsset};

mod assets;
#[cfg(feature = "gltf_animation")]
mod animations;
mod materials;
mod meshes;
mod nodes;
mod collisions;
mod options;
mod scene;
mod textures;

use assets::*;
#[cfg(feature = "gltf_animation")]
use animations::*;
use materials::*;
use meshes::*;
use nodes::*;
use collisions::*;
pub use options::{GltfCollisionPolicy, GltfLoadOptions, GltfLoadReport};
pub use scene::{load_gltf_actor, load_gltf_scene, load_gltf_scene_with_options, load_gltf_static_map, load_gltf_static_map_actor};
use textures::*;
