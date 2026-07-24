use vetrace_core::{Engine, InputState};
use vetrace_render::{
    RenderAssets, RenderFrame, RenderPlugin, RenderSettings, RenderTarget,
    SceneRenderBackend,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use super::assets::{fetch_declared_text_assets, install_text_assets, PreloadedTextAssets};
use super::input::WebInputBridge;
use super::renderer::WebRenderer;

/// Browser `RenderTarget` that plugs into the normal `RenderPlugin`. This lets
/// a WebAssembly game use the same `AppBuilder`, plugins, render extraction,
/// and render stage as its desktop build.
pub struct WebRenderTarget {
    renderer: WebRenderer,
    input: WebInputBridge,
    pending_text_assets: Option<PreloadedTextAssets>,
}

impl WebRenderTarget {
    pub async fn from_canvas(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let pending_text_assets = fetch_declared_text_assets(&canvas).await?;
        let input = WebInputBridge::attach(&canvas)?;
        let renderer = WebRenderer::new(canvas).await?;
        Ok(Self {
            renderer,
            input,
            pending_text_assets: Some(pending_text_assets),
        })
    }

    pub fn backend_label(&self) -> &str {
        self.renderer.backend_label()
    }

    fn take_pending_text_assets(&mut self) -> PreloadedTextAssets {
        self.pending_text_assets.take().unwrap_or_default()
    }
}

impl RenderTarget for WebRenderTarget {
    fn begin_frame(&mut self, engine: &mut Engine) {
        if let Some(loaded) = self.pending_text_assets.take() {
            install_text_assets(engine, loaded);
        }
        if !engine.contains_resource::<InputState>() {
            engine.insert_resource(InputState::new());
        }
        if let Some(input) = engine.get_resource_mut::<InputState>() {
            self.input.begin_frame(input);
        }
        self.renderer.sync_surface_size();
        let (width, height) = self.renderer.dimensions();
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.width = width;
            settings.height = height;
        }
    }

    fn render(&mut self, frame: &RenderFrame, assets: Option<&RenderAssets>) {
        let _ = self.renderer.render(frame, assets);
    }
}

/// Creates a normal Vetrace render plugin backed by an HTML canvas. Add the
/// returned plugin to the same `AppBuilder` used by desktop builds.
pub async fn web_render_plugin(canvas_id: &str) -> Result<RenderPlugin, JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("missing #{canvas_id}")))?
        .dyn_into::<HtmlCanvasElement>()?;
    let mut target = WebRenderTarget::from_canvas(canvas).await?;
    let mut text_assets = Some(target.take_pending_text_assets());
    let mut backend = Some(SceneRenderBackend::with_target(Box::new(target)));
    Ok(RenderPlugin::with_backend(move |engine| {
        if let Some(loaded) = text_assets.take() {
            install_text_assets(engine, loaded);
        }
        backend.take().unwrap_or_else(SceneRenderBackend::headless)
    }))
}
