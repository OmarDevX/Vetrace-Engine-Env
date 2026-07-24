use super::*;

// Shared egui overlay helpers for `wgpu_window.rs`.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    pub(super) fn egui_color(rgb: Vec3, alpha: f32) -> egui::Color32 {
        let rgba = (rgb.clamp(Vec3::ZERO, Vec3::ONE) * 255.0).round();
        egui::Color32::from_rgba_unmultiplied(
            rgba.x as u8,
            rgba.y as u8,
            rgba.z as u8,
            (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }
}

pub(super) fn push_pointer_button_event(
    events: &mut Vec<egui::Event>,
    pos: egui::Pos2,
    button: egui::PointerButton,
    pressed: bool,
    released: bool,
    modifiers: egui::Modifiers,
) {
    if pressed {
        events.push(egui::Event::PointerButton { pos, button, pressed: true, modifiers });
    }
    if released {
        events.push(egui::Event::PointerButton { pos, button, pressed: false, modifiers });
    }
}
