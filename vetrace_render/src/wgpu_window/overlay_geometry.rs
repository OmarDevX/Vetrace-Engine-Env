use super::*;

// Platform-neutral conversion of screen-space overlays into GPU vertices.

pub(super) fn overlay_vertices(overlays: &[RenderOverlayRect], width: f32, height: f32) -> Vec<OverlayVertex> {
    let mut sorted = overlays.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|overlay| overlay.rect.z_order);
    let mut vertices = Vec::with_capacity(sorted.len() * 6);
    for overlay in sorted {
        let center = Vec2::new(overlay.rect.anchor.x * width, overlay.rect.anchor.y * height) + overlay.rect.offset_px;
        let size = overlay.rect.size_px.max(Vec2::splat(1.0));
        let min = center - size * 0.5;
        let max = center + size * 0.5;
        let to_ndc = |p: Vec2| -> [f32; 2] {
            [p.x / width.max(1.0) * 2.0 - 1.0, 1.0 - p.y / height.max(1.0) * 2.0]
        };
        let c = (overlay.material.base_color + overlay.material.emissive).clamp(Vec3::ZERO, Vec3::ONE);
        let color = [c.x, c.y, c.z, overlay.material.alpha.clamp(0.0, 1.0)];
        let a = OverlayVertex { position: to_ndc(Vec2::new(min.x, min.y)), color };
        let b = OverlayVertex { position: to_ndc(Vec2::new(max.x, min.y)), color };
        let c_v = OverlayVertex { position: to_ndc(Vec2::new(max.x, max.y)), color };
        let d = OverlayVertex { position: to_ndc(Vec2::new(min.x, max.y)), color };
        vertices.extend_from_slice(&[a, b, c_v, a, c_v, d]);
    }
    vertices
}
