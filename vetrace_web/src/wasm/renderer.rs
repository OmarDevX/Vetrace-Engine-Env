use vetrace_render::{
    AdapterPreference, PresentModePreference, RenderAssets, RenderFrame, WgpuRenderer,
};
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

/// Browser platform adapter around the same full WGPU renderer used by native
/// Vetrace applications. It owns only the canvas lifecycle; all GPU pipelines
/// and render features live in `vetrace_render::WgpuRenderer`.
pub struct WebRenderer {
    canvas: HtmlCanvasElement,
    renderer: WgpuRenderer,
}

impl WebRenderer {
    pub async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        super::webgpu_compat::install();
        let (width, height, pixel_scale) = display_metrics(&canvas);
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let mut renderer = WgpuRenderer::from_surface(
            instance,
            surface,
            width,
            height,
            PresentModePreference::Fifo,
            browser_adapter_preference(&canvas),
        )
        .await
        .map_err(|error| JsValue::from_str(&error))?;
        renderer.set_pixel_scale_factor(pixel_scale);

        Ok(Self { canvas, renderer })
    }

    pub fn render(
        &mut self,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
    ) -> Result<bool, JsValue> {
        self.sync_surface_size();
        Ok(self.renderer.render_frame(frame, assets))
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.renderer.dimensions()
    }

    pub fn backend_label(&self) -> &str {
        self.renderer.backend_label()
    }

    pub fn sync_surface_size(&mut self) {
        let (width, height, pixel_scale) = display_metrics(&self.canvas);
        self.renderer.set_pixel_scale_factor(pixel_scale);
        if self.canvas.width() != width || self.canvas.height() != height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.renderer.resize(width, height);
        }
    }
}


fn browser_adapter_preference(canvas: &HtmlCanvasElement) -> AdapterPreference {
    match canvas.dataset().get("gpuPreference").as_deref() {
        Some("high-performance") => AdapterPreference::HighPerformance,
        _ => AdapterPreference::LowPower,
    }
}

fn display_metrics(canvas: &HtmlCanvasElement) -> (u32, u32, f32) {
    let pixel_ratio = web_sys::window()
        .map(|window| window.device_pixel_ratio())
        .unwrap_or(1.0);
    let width = ((canvas.client_width().max(1) as f64) * pixel_ratio)
        .round()
        .clamp(1.0, u32::MAX as f64) as u32;
    let height = ((canvas.client_height().max(1) as f64) * pixel_ratio)
        .round()
        .clamp(1.0, u32::MAX as f64) as u32;
    (width, height, pixel_ratio as f32)
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
