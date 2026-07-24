use std::cell::RefCell;
use std::rc::Rc;

use vetrace_core::{App, AppRunner};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Drives a normal Vetrace `AppRunner` from `requestAnimationFrame`.
///
/// Pair this with `web_render_plugin` and the desktop game can keep the same
/// `App`, systems, plugins, fixed-step physics, cleanup, and render stages.
pub async fn run_app<A: App + 'static>(mut runner: AppRunner<A>) -> Result<(), JsValue> {
    runner
        .initialize()
        .map_err(|error| JsValue::from_str(&format!("failed to initialize Vetrace app: {error}")))?;

    let state = Rc::new(RefCell::new(WebAppRunner {
        runner,
        last_timestamp_ms: None,
    }));
    let callback: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let callback_for_frame = Rc::clone(&callback);
    let state_for_frame = Rc::clone(&state);

    *callback.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp_ms: f64| {
        let result = state_for_frame.borrow_mut().frame(timestamp_ms);
        if let Err(error) = result {
            web_sys::console::error_1(&error);
            return;
        }
        let running = state_for_frame.borrow().runner.engine().is_running();
        if !running {
            state_for_frame.borrow_mut().runner.shutdown();
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

struct WebAppRunner<A: App> {
    runner: AppRunner<A>,
    last_timestamp_ms: Option<f64>,
}

impl<A: App> WebAppRunner<A> {
    fn frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        let dt = self
            .last_timestamp_ms
            .map(|previous| ((timestamp_ms - previous) / 1000.0) as f32)
            .unwrap_or(1.0 / 60.0)
            .clamp(1.0 / 240.0, 0.1);
        self.last_timestamp_ms = Some(timestamp_ms);
        self.runner
            .run_frame(dt)
            .map_err(|error| JsValue::from_str(&format!("Vetrace frame failed: {error}")))
    }
}
