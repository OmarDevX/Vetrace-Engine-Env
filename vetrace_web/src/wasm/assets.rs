use vetrace_core::Engine;
use vetrace_render::RenderAssets;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlCanvasElement, Response};

pub type PreloadedTextAssets = Vec<(String, String)>;

/// Loads path-addressable text assets declared by the page. The canvas
/// attribute uses comma-separated
/// `engine/path.wgsl=https://.../file.wgsl` entries.
pub async fn fetch_declared_text_assets(
    canvas: &HtmlCanvasElement,
) -> Result<PreloadedTextAssets, JsValue> {
    let Some(declaration) = canvas.dataset().get("textAssets") else { return Ok(Vec::new()); };
    let mut loaded = Vec::new();
    for entry in declaration.split(',').map(str::trim).filter(|entry| !entry.is_empty()) {
        let (logical_path, url) = entry
            .split_once('=')
            .map(|(path, url)| (path.trim(), url.trim()))
            .unwrap_or((entry, entry));
        if logical_path.is_empty() || url.is_empty() {
            continue;
        }
        loaded.push((logical_path.to_string(), fetch_text(url).await?));
    }
    Ok(loaded)
}

pub fn install_text_assets(engine: &mut Engine, loaded: PreloadedTextAssets) {
    if loaded.is_empty() {
        return;
    }
    if !engine.contains_resource::<RenderAssets>() {
        engine.insert_resource(RenderAssets::default());
    }
    let Some(assets) = engine.get_resource_mut::<RenderAssets>() else { return; };
    for (path, source) in loaded {
        assets.insert_text_asset(path, source);
    }
}

pub async fn preload_declared_text_assets(
    canvas: &HtmlCanvasElement,
    engine: &mut Engine,
) -> Result<(), JsValue> {
    let loaded = fetch_declared_text_assets(canvas).await?;
    install_text_assets(engine, loaded);
    Ok(())
}

pub async fn fetch_text(url: &str) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
    let response = JsFuture::from(window.fetch_with_str(url))
        .await?
        .dyn_into::<Response>()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "asset request failed with HTTP {} for {url}",
            response.status()
        )));
    }
    let text = JsFuture::from(response.text()?).await?;
    text.as_string()
        .ok_or_else(|| JsValue::from_str(&format!("asset response was not text: {url}")))
}
