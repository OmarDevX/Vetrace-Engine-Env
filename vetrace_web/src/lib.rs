//! Browser runtime for Vetrace examples.
//!
//! Browsers require an externally driven animation loop and DOM input, so this
//! crate provides only those platform adapters. GPU rendering, render-frame
//! extraction, materials, lighting, post-processing, and runtime UI all remain
//! in the shared `vetrace_render::WgpuRenderer`.

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::{run_app, run_web, start, start_example, web_render_plugin, WebGame, WebRenderTarget};

/// Native builds keep a small stub so `cargo test --workspace` remains valid.
#[cfg(not(target_arch = "wasm32"))]
pub fn browser_runtime_available() -> bool { false }
