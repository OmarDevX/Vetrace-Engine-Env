//! Lightweight WGPU building blocks for the active renderer.
//!
//! This is intentionally **not** the old monolithic renderer. It provides the
//! two GPU pieces the active API needs first:
//!
//! - compile/cache game-provided WGSL custom material shaders and bind their
//!   uniform params;
//! - prepare a mask-based outline fullscreen pass that a future WGPU target can
//!   call after drawing entity/object masks.
//!
//! The SDL software target keeps working without this feature. Enabling the
//! `wgpu_render` feature makes these types available to a WGPU target or game
//! runtime without coupling `vetrace_core` to WGPU.

use std::borrow::Cow;
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

mod custom_shader_uniform;
mod outline_pass;
mod outline_shader;
mod shader_cache;

pub use custom_shader_uniform::*;
pub use outline_pass::*;
pub use outline_shader::*;
pub use shader_cache::*;
