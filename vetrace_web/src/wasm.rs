mod application;
mod app_runner;
mod assets;
mod examples;
mod input;
mod renderer;
mod target;
mod webgpu_compat;

use wasm_bindgen::prelude::*;

pub use app_runner::run_app;
pub use application::{run_web, start_example, WebGame};
pub use target::{web_render_plugin, WebRenderTarget};

/// Starts the example selected by `?example=<slug>` or by the canvas
/// `data-example` attribute. This is an explicit export rather than a
/// `#[wasm_bindgen(start)]` hook, so games that depend on `vetrace_web` can
/// define their own WebAssembly entry point without the gallery auto-starting.
#[wasm_bindgen]
pub async fn start() -> Result<(), JsValue> {
    application::start_from_page().await
}
