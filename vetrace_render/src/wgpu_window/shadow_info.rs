use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn shadow_info_for_frame(frame: &RenderFrame, shadow_light: Option<(Vec3, ShadowMode)>, shadow_candidates: &[ShadowCandidate]) -> ShadowInfo {
    let Some((direction, mode)) = shadow_light else {
        return disabled_shadow_info(&frame.settings);
    };
    if shadow_candidates.is_empty() {
        return disabled_shadow_info(&frame.settings);
    }

    let light_dir = direction.normalize_or_zero();
    let light_dir = if light_dir.length_squared() > 0.0 { light_dir } else { Vec3::new(-0.35, -1.0, -0.25).normalize() };
    let cascade_count = normalize_shadow_cascade_count(frame.settings.shadow_cascade_count);
    let shadow_distance = if frame.settings.shadow_max_distance > 0.0 {
        frame.settings.shadow_max_distance.min(frame.camera.far).max(frame.camera.near + 0.1)
    } else {
        frame.camera.far.min(500.0).max(frame.camera.near + 0.1)
    };
    let splits = cascade_split_distances(frame.camera.near.max(0.01), shadow_distance, cascade_count);
    let mut matrices = [Mat4::IDENTITY; SHADOW_CASCADE_COUNT];

    let mut split_near = frame.camera.near.max(0.01);
    for cascade_index in 0..cascade_count {
        let split_far = splits[cascade_index].max(split_near + 0.1);
        matrices[cascade_index] = directional_cascade_matrix(
            frame,
            light_dir,
            shadow_candidates,
            split_near,
            split_far,
            normalize_shadow_map_size(frame.settings.shadow_map_size),
        );
        split_near = split_far;
    }

    let requested_filter_mode = match mode {
        ShadowMode::None => ShadowFilterMode::Hard,
        ShadowMode::Hard => ShadowFilterMode::Hard,
        ShadowMode::Soft => {
            // Keep old configs that disabled PCSS from unexpectedly using it when
            // they deserialize without the newer explicit filter-mode field.
            if frame.settings.shadow_filter_mode == ShadowFilterMode::Pcss && !frame.settings.shadow_pcss {
                ShadowFilterMode::Pcf
            } else {
                frame.settings.shadow_filter_mode
            }
        }
    };
    let filter_mode = if mode == ShadowMode::Soft { requested_filter_mode } else { ShadowFilterMode::Hard };
    let bias = frame.settings.shadow_bias.max(if mode == ShadowMode::Soft { 0.0014 } else { 0.0010 });
    let soft_radius = if filter_mode.uses_soft_radius() { frame.settings.shadow_soft_radius.max(0.0) } else { 0.0 };
    ShadowInfo {
        enabled: true,
        soft_radius,
        view_proj: matrices,
        cascade_splits: splits,
        cascade_count,
        bias,
        slope_bias: frame.settings.shadow_slope_bias.max(0.0),
        normal_bias: frame.settings.shadow_normal_bias.max(0.0),
        pcf_quality: frame.settings.shadow_pcf_quality.clamp(1, 3) as f32,
        filter_mode,
        pcss_light_radius: frame.settings.shadow_pcss_light_radius.max(0.0),
        evsm_blur_radius: frame.settings.shadow_evsm_blur_radius.max(0.0),
        evsm_exponent: frame.settings.shadow_evsm_exponent.clamp(1.0, 5.5),
    }
}
