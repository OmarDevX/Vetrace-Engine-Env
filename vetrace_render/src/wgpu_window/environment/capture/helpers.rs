use super::*;

pub(super) fn reflection_signature_mix(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(0x100000001b3);
}

pub(super) fn reflection_signature_vec3(hash: &mut u64, value: Vec3) {
    reflection_signature_mix(hash, value.x.to_bits() as u64);
    reflection_signature_mix(hash, value.y.to_bits() as u64);
    reflection_signature_mix(hash, value.z.to_bits() as u64);
}

pub(super) fn reflection_signature_mat4(hash: &mut u64, value: Mat4) {
    for component in value.to_cols_array() {
        reflection_signature_mix(hash, component.to_bits() as u64);
    }
}

pub(super) fn reflection_probe_scene_signature(frame: &RenderFrame, probe: &RenderReflectionProbe) -> u64 {
    let mut hash = frame.reflection_global_signature;
    reflection_signature_mix(&mut hash, probe.capture_include_layers as u64);
    reflection_signature_mix(&mut hash, probe.capture_exclude_layers as u64);
    reflection_signature_vec3(&mut hash, probe.capture_position_world);
    reflection_signature_mat4(&mut hash, probe.probe_to_world);
    reflection_signature_vec3(&mut hash, probe.half_extents);
    reflection_signature_mix(&mut hash, probe.capture_near.to_bits() as u64);
    reflection_signature_mix(&mut hash, probe.capture_far.to_bits() as u64);
    reflection_signature_mix(&mut hash, probe.invalidation_delay_seconds.to_bits() as u64);
    reflection_signature_mix(&mut hash, probe.capture_transparent as u64);
    reflection_signature_mix(&mut hash, probe.capture_shadows as u64);
    reflection_signature_mix(&mut hash, probe.capture_custom_materials as u64);
    reflection_signature_mix(&mut hash, probe.capture_resolution as u64);
    let layer_mask = probe.capture_include_layers & !probe.capture_exclude_layers;
    for bit in 0..32 {
        if layer_mask & (1_u32 << bit) != 0 {
            reflection_signature_mix(&mut hash, frame.reflection_layer_signatures[bit]);
        }
    }
    hash
}

pub(super) fn reflection_probe_capture_due(
    probe: &RenderReflectionProbe,
    state: &ReflectionProbeCaptureState,
    now: Instant,
    scene_signature: u64,
) -> bool {
    use crate::components::{ReflectionProbeCaptureMode, ReflectionProbeInvalidationMode};
    let scene_changed = matches!(probe.invalidation_mode, ReflectionProbeInvalidationMode::SceneChanges)
        && state.completed_scene_signature != scene_signature
        && state.observed_scene_signature == scene_signature
        && state.scene_change_observed_at.is_some_and(|observed_at| {
            now.duration_since(observed_at).as_secs_f32() >= probe.invalidation_delay_seconds
        });
    match probe.capture_mode {
        ReflectionProbeCaptureMode::Imported => false,
        ReflectionProbeCaptureMode::Baked | ReflectionProbeCaptureMode::OnDemand => {
            !state.has_capture || state.completed_revision != probe.capture_revision || scene_changed
        }
        ReflectionProbeCaptureMode::Realtime => {
            !state.has_capture
                || state.completed_revision != probe.capture_revision
                || scene_changed
                || state.last_completed.map_or(true, |last| {
                    now.duration_since(last).as_secs_f32() >= probe.update_interval_seconds
                })
        }
    }
}

pub(super) fn reflection_capture_face_axes(face: u32) -> (Vec3, Vec3) {
    match face {
        0 => (Vec3::X, -Vec3::Y),
        1 => (-Vec3::X, -Vec3::Y),
        2 => (Vec3::Y, Vec3::Z),
        3 => (-Vec3::Y, -Vec3::Z),
        4 => (Vec3::Z, -Vec3::Y),
        _ => (-Vec3::Z, -Vec3::Y),
    }
}

pub(super) fn reflection_capture_camera(probe: &RenderReflectionProbe, face: u32) -> Camera {
    let (local_direction, local_up) = reflection_capture_face_axes(face);
    let direction = probe.probe_to_world.transform_vector3(local_direction).normalize_or_zero();
    let up = probe.probe_to_world.transform_vector3(local_up).normalize_or_zero();
    Camera {
        position: probe.capture_position_world,
        target: probe.capture_position_world + direction,
        up,
        fov_y_radians: 90.0_f32.to_radians(),
        near: probe.capture_near,
        far: probe.capture_far,
    }
}


#[cfg(test)]
mod reflection_capture_tests {
    use super::*;

    #[test]
    fn cubemap_capture_axes_are_orthonormal_and_unique() {
        let mut directions = Vec::new();
        for face in 0..6 {
            let (direction, up) = reflection_capture_face_axes(face);
            assert!((direction.length() - 1.0).abs() < 1.0e-6);
            assert!((up.length() - 1.0).abs() < 1.0e-6);
            assert!(direction.dot(up).abs() < 1.0e-6);
            assert!(!directions.contains(&direction));
            directions.push(direction);
        }
    }
}
