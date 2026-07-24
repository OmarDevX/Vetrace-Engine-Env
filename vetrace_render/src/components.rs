use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

// Renderer-facing component entry point. Each family is a real Rust module so
// imports, visibility, and compile errors stay local to the file that owns them.
#[path = "component_handles.rs"]
mod handles;
#[path = "component_gltf_collision.rs"]
mod gltf_collision;
#[path = "component_material.rs"]
mod material;
#[path = "component_primitives.rs"]
mod primitives;
#[path = "component_custom_shader.rs"]
mod custom_shader;
#[path = "component_render_texture.rs"]
mod render_texture;
#[path = "component_environment.rs"]
mod environment;
#[path = "component_lighting.rs"]
mod lighting;
#[path = "component_baked_lighting.rs"]
mod baked_lighting;
#[path = "component_post_processing.rs"]
mod post_processing;
#[cfg(feature = "render_2d")]
#[path = "component_2d.rs"]
mod render_2d;

pub use baked_lighting::*;
pub use custom_shader::*;
pub use environment::*;
pub use gltf_collision::*;
pub use handles::*;
pub use lighting::*;
pub use material::*;
pub use post_processing::*;
pub use primitives::*;
pub use render_texture::*;
#[cfg(feature = "render_2d")]
pub use render_2d::*;
