use std::cell::RefCell;
use std::rc::Rc;

use vetrace_core::{Engine, InputState, Stage};
use vetrace_render::{build_render_frame, RenderAssets, RenderSettings};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, UrlSearchParams};

use super::assets::preload_declared_text_assets;
use super::examples::{ExampleKind, ExampleScene};
use super::input::WebInputBridge;
use super::renderer::WebRenderer;

/// Game-side hook used by Rust applications compiled to WebAssembly. The game
/// owns normal Vetrace ECS setup and update logic; the browser runner only
/// supplies input, frame timing, canvas lifecycle, and the shared renderer.
pub trait WebGame: 'static {
    fn update(&mut self, engine: &mut Engine, time_seconds: f32, delta_seconds: f32);
}

impl WebGame for ExampleScene {
    fn update(&mut self, engine: &mut Engine, time_seconds: f32, delta_seconds: f32) {
        ExampleScene::update(self, engine, time_seconds, delta_seconds);
    }
}

/// Runs a normal Vetrace `Engine` and game state on a browser canvas. Games can
/// use the same actor/component setup as desktop builds and select only a
/// different platform runner at their WebAssembly entry point.
pub async fn run_web<G: WebGame>(
    canvas_id: &str,
    mut engine: Engine,
    game: G,
) -> Result<(), JsValue> {
    install_panic_hook();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("missing #{canvas_id}")))?
        .dyn_into::<HtmlCanvasElement>()?;
    preload_declared_text_assets(&canvas, &mut engine).await?;
    launch_engine(canvas, engine, game).await
}

pub async fn start_from_page() -> Result<(), JsValue> {
    install_panic_hook();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .get_element_by_id("vetrace-canvas")
        .ok_or_else(|| JsValue::from_str("missing #vetrace-canvas"))?
        .dyn_into::<HtmlCanvasElement>()?;
    let example = selected_example(&canvas);
    launch(canvas, &example).await
}

#[wasm_bindgen]
pub async fn start_example(canvas_id: String, example: String) -> Result<(), JsValue> {
    install_panic_hook();
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document unavailable"))?;
    let canvas = document
        .get_element_by_id(&canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("missing #{canvas_id}")))?
        .dyn_into::<HtmlCanvasElement>()?;
    launch(canvas, &example).await
}

async fn launch(canvas: HtmlCanvasElement, example: &str) -> Result<(), JsValue> {
    let kind = ExampleKind::from_slug(example);
    let (mut engine, scene) = ExampleScene::build(kind).map_err(|error| JsValue::from_str(&error))?;
    preload_declared_text_assets(&canvas, &mut engine).await?;
    launch_engine(canvas, engine, scene).await
}

async fn launch_engine<G: WebGame>(
    canvas: HtmlCanvasElement,
    engine: Engine,
    game: G,
) -> Result<(), JsValue> {
    set_status("Initializing the shared Vetrace WGPU renderer…", false);
    let input = WebInputBridge::attach(&canvas)?;
    let renderer = WebRenderer::new(canvas).await?;
    set_backend(renderer.backend_label());
    let application = Rc::new(RefCell::new(WebApplication {
        engine,
        game,
        renderer,
        input,
        last_timestamp_ms: None,
        first_present_reported: false,
    }));
    set_status("Running · shared desktop/browser renderer", false);
    start_animation_loop(application)
}

struct WebApplication<G: WebGame> {
    engine: Engine,
    game: G,
    renderer: WebRenderer,
    input: WebInputBridge,
    last_timestamp_ms: Option<f64>,
    first_present_reported: bool,
}

impl<G: WebGame> WebApplication<G> {
    fn frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        let dt = self
            .last_timestamp_ms
            .map(|previous| ((timestamp_ms - previous) / 1000.0) as f32)
            .unwrap_or(1.0 / 60.0)
            .clamp(1.0 / 240.0, 0.1);
        self.last_timestamp_ms = Some(timestamp_ms);
        let time = (timestamp_ms / 1000.0) as f32;

        if let Some(input) = self.engine.get_resource_mut::<InputState>() {
            self.input.begin_frame(input);
        }
        self.game.update(&mut self.engine, time, dt);
        self.renderer.sync_surface_size();
        let (width, height) = self.renderer.dimensions();
        if let Some(settings) = self.engine.get_resource_mut::<RenderSettings>() {
            settings.time_seconds = time;
            settings.width = width;
            settings.height = height;
        }
        self.engine.run_stage(Stage::PostUpdate, dt);
        self.engine.run_stage(Stage::RenderExtract, dt);
        let frame = build_render_frame(&self.engine);
        if frame.objects.is_empty() {
            return Err(JsValue::from_str(
                "The selected example extracted zero render objects. The browser runner is active, but the example scene was not installed.",
            ));
        }
        let assets = self.engine.get_resource::<RenderAssets>();
        let presented = self.renderer.render(&frame, assets)?;
        if !presented {
            set_status("Waiting for the WebGPU canvas surface…", false);
            return Ok(());
        }
        if !self.first_present_reported {
            set_status(
                &format!(
                    "Running · {} objects · {} directional / {} point lights · {}×{}",
                    frame.objects.len(),
                    frame.directional_lights.len(),
                    frame.point_lights.len(),
                    width,
                    height,
                ),
                false,
            );
            self.first_present_reported = true;
        }
        Ok(())
    }
}

fn start_animation_loop<G: WebGame>(application: Rc<RefCell<WebApplication<G>>>) -> Result<(), JsValue> {
    let callback: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let callback_for_frame = Rc::clone(&callback);
    let application_for_frame = Rc::clone(&application);

    *callback.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp_ms: f64| {
        if let Err(error) = application_for_frame.borrow_mut().frame(timestamp_ms) {
            web_sys::console::error_1(&error);
            set_status(&format!("Runtime error: {}", js_value_message(&error)), true);
            return;
        }
        if let Some(window) = web_sys::window() {
            if let Some(callback) = callback_for_frame.borrow().as_ref() {
                let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }
    }) as Box<dyn FnMut(f64)>));

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let borrowed = callback.borrow();
    let callback = borrowed
        .as_ref()
        .ok_or_else(|| JsValue::from_str("animation callback unavailable"))?;
    window.request_animation_frame(callback.as_ref().unchecked_ref())?;
    Ok(())
}

fn selected_example(canvas: &HtmlCanvasElement) -> String {
    let from_query = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|query| UrlSearchParams::new_with_str(&query).ok())
        .and_then(|params| params.get("example"));
    from_query
        .or_else(|| canvas.dataset().get("example"))
        .unwrap_or_else(|| "rotating-cube".to_string())
}

fn set_backend(label: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else { return; };
    let Ok(Some(element)) = document.query_selector("[data-backend]") else { return; };
    element.set_text_content(Some(label));
}

fn set_status(message: &str, is_error: bool) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else { return; };
    let Some(element) = document.get_element_by_id("runtime-status") else { return; };
    element.set_text_content(Some(message));
    element.set_class_name(if is_error { "runtime-status error" } else { "runtime-status" });
}

fn js_value_message(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let message = panic_info.to_string();
        web_sys::console::error_1(&JsValue::from_str(&message));
        set_status(&format!("Engine panic: {message}"), true);
    }));
}
