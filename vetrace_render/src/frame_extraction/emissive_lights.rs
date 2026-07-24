use super::*;

pub(super) fn push_emissive_point_lights(
    emitter: &EmissiveLightEmitter,
    material: &Material,
    transform: &GlobalTransform,
    point_lights: &mut Vec<RenderPointLight>,
) {
    if !emitter.enabled || emitter.intensity <= 0.0 || emitter.range <= 0.0 {
        return;
    }

    let emissive = material.emissive.max(Vec3::ZERO);
    let peak = emissive.x.max(emissive.y).max(emissive.z);
    if peak <= 1.0e-4 {
        return;
    }

    let color = emissive / peak;
    let intensity = peak * emitter.intensity.max(0.0);
    let local_axis = if emitter.local_axis.length_squared() > 1.0e-8 {
        emitter.local_axis.normalize()
    } else {
        Vec3::Z
    };
    let scaled_axis = Vec3::new(
        local_axis.x * transform.scale.x,
        local_axis.y * transform.scale.y,
        local_axis.z * transform.scale.z,
    );
    let world_axis = transform.rotation * scaled_axis;
    let sample_count = emitter.samples.clamp(1, 4) as usize;
    let length = emitter.length.max(0.0);

    for index in 0..sample_count {
        let centered = if sample_count == 1 {
            0.0
        } else {
            (index as f32 + 0.5) / sample_count as f32 - 0.5
        };
        point_lights.push(RenderPointLight {
            position: transform.translation + world_axis * (length * centered),
            color,
            intensity,
            range: Some(emitter.range),
            shadow_mode: ShadowMode::None,
        });
    }
}
