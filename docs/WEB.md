# Vetrace on the web

Vetrace uses one WGPU renderer on desktop and in the browser. The browser crate no longer owns a second mesh pipeline or reduced WGSL shader. It creates a canvas surface and passes it to `vetrace_render::WgpuRenderer`, which owns the same geometry caches, materials, custom shaders, shadows, SSAO, SSR, render textures, reflection probes, environment lighting, bloom/FXAA, atmosphere/fog, baked-lighting, and runtime egui/UI paths used by native WGPU builds.

Only platform services differ:

- Desktop drives a native `winit` event loop, native cursor policy, and a window surface.
- Browser drives `requestAnimationFrame`, DOM input events, high-DPI canvas resizing, and a canvas surface.
- Native file-backed shader paths may read from disk. Browser pages preload those paths into `RenderAssets::text_assets` with `data-text-assets`.
- Browser networking/audio still require web transports and WebAudio backends; those are not renderer differences.

## Run the same App on desktop and web

A game can keep its normal `App`, plugins, systems, fixed physics stages, and render extraction. Only the renderer plugin and runner are selected at the platform entry point.

```rust
use vetrace_core::{AppBuilder, Engine};
use vetrace_render::RenderPlugin;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .add_plugin(RenderPlugin::new())
        .run_until_stopped(MyGame::default(), None, 1.0 / 60.0)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    let render = vetrace_web::web_render_plugin("vetrace-canvas").await?;
    let runner = AppBuilder::new()
        .add_plugin(render)
        .build(MyGame::default());
    vetrace_web::run_app(runner).await
}
```

`web_render_plugin` returns the normal `vetrace_render::RenderPlugin`, backed by a browser `RenderTarget`. `run_app` advances the normal `AppRunner` from `requestAnimationFrame`.

## Browser asset paths

Custom materials and post-process passes can keep normal `asset_path` values. Declare browser URLs on the canvas:

```html
<canvas
  id="vetrace-canvas"
  data-text-assets="assets/water.wgsl=/assets/water.wgsl,assets/outline.wgsl=/assets/outline.wgsl"
></canvas>
```

The browser adapter fetches the files before rendering and installs them into the ordinary `RenderAssets` resource. Inline `wgsl_source` continues to work unchanged.

Meshes, textures, cubemaps, and baked-lighting data already render through the shared renderer once their decoded assets are inserted into `RenderAssets`. HTTP/package decoding belongs to the browser asset/runtime layer rather than the GPU renderer.

## WebGPU requirement

The full renderer targets browser WebGPU. A separate reduced WebGL renderer is intentionally not maintained because it would reintroduce feature drift. Browsers without WebGPU receive a clear startup error instead of silently running a different renderer with missing effects.

### Adapter preference on Linux

The browser adapter defaults to `low-power`. On hybrid Linux systems this normally selects the integrated GPU and avoids a Dawn/Vulkan external-memory import failure observed on some discrete-GPU canvas paths. The browser preference is a hint, so the browser may still select another adapter when no integrated adapter is available.

The examples page supports an explicit override:

```text
http://127.0.0.1:8080/website/examples/?example=rotating-cube&gpu=high-performance
```

Custom pages can select the same override before starting Vetrace:

```html
<canvas id="vetrace-canvas" data-gpu-preference="high-performance"></canvas>
```

Leave the attribute unset for the compatibility-first low-power default.

## Build and run

Install the WebAssembly target and `wasm-pack`:

```bash
cargo install wasm-pack
rustup target add wasm32-unknown-unknown
```

From the workspace root:

```bash
./scripts/build_web.sh
./scripts/serve_web.sh
```

Open:

```text
http://127.0.0.1:8080/website/
http://127.0.0.1:8080/website/examples/
```

The site must be served over HTTP. Opening pages through `file://` prevents ES modules and WebAssembly from loading correctly.

## Validation

```bash
python3 scripts/validate_web_port.py
cargo check -p vetrace_render --features "wgpu_render egui_render" --target wasm32-unknown-unknown
cargo check -p vetrace_web --target wasm32-unknown-unknown
cargo test --workspace
```

The static validator rejects a separate browser render pipeline and verifies that the examples metadata matches the Rust runtime.

## Add an example

1. Add a variant and scene construction in `vetrace_web/src/wasm/examples.rs`.
2. Add matching metadata and source in `website/assets/examples-data.js`.
3. Run `python3 scripts/validate_web_port.py`.
4. Rebuild the WebAssembly package.

## Remaining platform work

These are browser platform/runtime tasks, not missing renderer features:

- HTTP or `.vpak` loading for complete projects and imported assets.
- WebSocket/WebTransport/WebRTC implementations of the networking transport interface; browsers cannot open UDP sockets.
- WebAudio backend and user-gesture activation.
- Browser persistence through IndexedDB or the File System Access API.
- Browser-compatible Lua script/mod loading and debugger transport.
- A browser version of Studio would additionally need browser file handles, workers, and remote or embedded compilation.
