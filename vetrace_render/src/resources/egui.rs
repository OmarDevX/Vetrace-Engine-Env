use serde::{Deserialize, Serialize};

#[cfg(feature = "egui_render")]
use glam::Vec2;
#[cfg(feature = "egui_render")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "egui_render")]
use super::Camera;

/// Lightweight data-only egui overlay request.
///
/// This keeps editor/UI crates from depending on the WGPU renderer internals.
/// Producers such as `vetrace_editor` write this resource; the active WGPU
/// target renders it with `egui_wgpu` when the `egui_overlay` feature is enabled.

/// Minimal renderer-neutral input snapshot for the WGPU egui overlay.
///
/// `vetrace_core::InputState` intentionally stays platform/UI agnostic.  The
/// render backend copies only the frame-local mouse/modifier state needed by
/// egui so the overlay can receive clicks and drags without making core depend
/// on egui or winit. Coordinates are physical window pixels.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct EguiOverlayInputSnapshot {
    pub mouse_position: [f32; 2],
    pub mouse_wheel_delta: [f32; 2],
    pub left_pressed: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub right_released: bool,
    pub middle_pressed: bool,
    pub middle_released: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Renderer-neutral keyboard transition passed to egui tools.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EguiOverlayKeyEvent {
    pub key: String,
    pub pressed: bool,
}

/// Text and key transitions are kept separate from the pointer snapshot so the
/// established lightweight pointer type remains `Copy` for existing Rust tools.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EguiOverlayKeyboardInputSnapshot {
    #[serde(default)]
    pub text_input: String,
    #[serde(default)]
    pub key_events: Vec<EguiOverlayKeyEvent>,
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Debug)]
pub struct EguiToolContext {
    pub screen_size_points: Vec2,
    pub surface_size_pixels: [u32; 2],
    pub pixels_per_point: f32,
    pub camera: Camera,
    pub input: Option<EguiOverlayInputSnapshot>,
    pub keyboard_input: Option<EguiOverlayKeyboardInputSnapshot>,
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum EguiToolLayer {
    /// Viewport-local tools such as transform gizmos and selection overlays.
    /// These are evaluated first so application chrome can always remain on top.
    Viewport,
    /// Normal application UI such as Studio panels and inspectors.
    #[default]
    Interface,
    /// Popups or tool UI that intentionally belongs above the main interface.
    Overlay,
}

#[cfg(feature = "egui_render")]
pub trait EguiTool: Send + 'static {
    /// Called by the active egui renderer while building the egui overlay for the frame.
    ///
    /// Tools should keep engine mutations out of this callback. The intended pattern is:
    /// game/tool system writes a request into shared tool state before rendering, this
    /// callback draws/interacts with egui and writes a small result into that shared state,
    /// then the game/tool system applies that result on the next update tick.
    fn ui(&mut self, ctx: &egui::Context, frame: &EguiToolContext);

    /// Controls deterministic composition between independently registered tools.
    /// Registration order is preserved within the same layer.
    fn layer(&self) -> EguiToolLayer { EguiToolLayer::Interface }
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Default)]
pub struct EguiToolRegistry {
    tools: Arc<Mutex<Vec<Box<dyn EguiTool>>>>,
}

#[cfg(feature = "egui_render")]
impl std::fmt::Debug for EguiToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EguiToolRegistry")
            .field("tool_count", &self.tool_count())
            .finish()
    }
}

#[cfg(feature = "egui_render")]
impl EguiToolRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register<T: EguiTool>(&self, tool: T) {
        if let Ok(mut tools) = self.tools.lock() {
            tools.push(Box::new(tool));
            // Stable sorting keeps registration order for tools in the same
            // layer while making viewport/interface composition independent of
            // plugin initialization order.
            tools.sort_by_key(|tool| tool.layer());
        }
    }

    pub fn clear(&self) {
        if let Ok(mut tools) = self.tools.lock() {
            tools.clear();
        }
    }

    pub fn tool_count(&self) -> usize {
        self.tools.lock().map(|tools| tools.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool { self.tool_count() == 0 }

    pub fn run(&self, ctx: &egui::Context, frame: &EguiToolContext) {
        if let Ok(mut tools) = self.tools.lock() {
            for tool in tools.iter_mut() {
                tool.ui(ctx, frame);
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EguiOverlayPanel {
    pub enabled: bool,
    pub title: String,
    pub subtitle: String,
    pub status: String,
    pub lines: Vec<String>,
    pub controls: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_snapshot_remains_copy_for_existing_tools() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<EguiOverlayInputSnapshot>();
    }


    #[cfg(feature = "egui_render")]
    #[test]
    fn tool_layers_are_deterministic_and_stable() {
        use std::sync::{Arc, Mutex};

        struct RecordingTool {
            name: &'static str,
            layer: EguiToolLayer,
            calls: Arc<Mutex<Vec<&'static str>>>,
        }

        impl EguiTool for RecordingTool {
            fn ui(&mut self, _ctx: &egui::Context, _frame: &EguiToolContext) {
                self.calls.lock().unwrap().push(self.name);
            }

            fn layer(&self) -> EguiToolLayer { self.layer }
        }

        let calls = Arc::new(Mutex::new(Vec::new()));
        let registry = EguiToolRegistry::new();
        for (name, layer) in [
            ("interface-a", EguiToolLayer::Interface),
            ("overlay", EguiToolLayer::Overlay),
            ("viewport-a", EguiToolLayer::Viewport),
            ("interface-b", EguiToolLayer::Interface),
            ("viewport-b", EguiToolLayer::Viewport),
        ] {
            registry.register(RecordingTool { name, layer, calls: calls.clone() });
        }

        registry.run(
            &egui::Context::default(),
            &EguiToolContext {
                screen_size_points: Vec2::new(1280.0, 720.0),
                surface_size_pixels: [1280, 720],
                pixels_per_point: 1.0,
                camera: Camera::default(),
                input: None,
                keyboard_input: None,
            },
        );

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["viewport-a", "viewport-b", "interface-a", "interface-b", "overlay"]
        );
    }
}
