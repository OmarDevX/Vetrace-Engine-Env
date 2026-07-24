use super::*;

// Direct-light and rectangular area-light evaluation.

#[allow(clippy::too_many_arguments)]
pub(super) fn direct_irradiance(

    position: Vec3,
    normal: Vec3,
    ignore_triangle: Option<usize>,
    triangles: &[BakeTriangle],
    directional: &[RenderDirectionalLight],
    points: &[RenderPointLight],
    spots: &[RenderSpotLight],
    areas: &[BakeRectAreaLight],
    bias: f32,
) -> Vec3 {
    let n = normal.normalize_or_zero();
    let origin = position + n * bias.max(0.0001);
    let mut out = Vec3::ZERO;
    for light in directional.iter().take(4) {
        let l = -light.direction.normalize_or_zero();
        let ndotl = n.dot(l).max(0.0);
        if ndotl <= 0.0 || trace_any(origin, l, f32::INFINITY, ignore_triangle, triangles) { continue; }
        out += light.color.max(Vec3::ZERO) * light.intensity.max(0.0) * ndotl;
    }
    for light in points.iter().take(8) {
        let to_light = light.position - position;
        let distance2 = to_light.length_squared().max(0.0001);
        let distance = distance2.sqrt();
        let l = to_light / distance;
        let ndotl = n.dot(l).max(0.0);
        if ndotl <= 0.0 || trace_any(origin, l, (distance - bias).max(0.0), ignore_triangle, triangles) { continue; }
        let range = light.range.unwrap_or(0.0);
        let attenuation = range_attenuation(distance, range) / distance2;
        out += light.color.max(Vec3::ZERO) * light.intensity.max(0.0) * attenuation * ndotl;
    }
    for light in spots.iter().take(4) {
        let to_light = light.position - position;
        let distance2 = to_light.length_squared().max(0.0001);
        let distance = distance2.sqrt();
        let l = to_light / distance;
        let ndotl = n.dot(l).max(0.0);
        if ndotl <= 0.0 || trace_any(origin, l, (distance - bias).max(0.0), ignore_triangle, triangles) { continue; }
        let theta = (-l).dot(light.direction.normalize_or_zero());
        let inner = light.inner_cone_angle.max(0.0).cos();
        let outer = light.outer_cone_angle.max(light.inner_cone_angle + 0.001).cos();
        let cone = ((theta - outer) / (inner - outer).max(0.001)).clamp(0.0, 1.0);
        let attenuation = range_attenuation(distance, light.range.unwrap_or(0.0)) * cone * cone / distance2;
        out += light.color.max(Vec3::ZERO) * light.intensity.max(0.0) * attenuation * ndotl;
    }
    for light in areas.iter().take(8) {
        out += rect_area_light_irradiance(
            position,
            n,
            origin,
            ignore_triangle,
            triangles,
            *light,
            bias,
        );
    }
    out
}

pub(super) fn rect_area_light_irradiance(
    position: Vec3,
    normal: Vec3,
    origin: Vec3,
    ignore_triangle: Option<usize>,
    triangles: &[BakeTriangle],
    light: BakeRectAreaLight,
    bias: f32,
) -> Vec3 {
    let sample_count = light.samples.clamp(1, 64);
    let area = light.width * light.height;
    let emitted = light.color.max(Vec3::ZERO) * light.intensity.max(0.0);
    let mut irradiance = Vec3::ZERO;

    for index in 0..sample_count {
        // Deterministic Hammersley points cover the full rectangle even when
        // the requested sample count is not a perfect square.
        let u = (index as f32 + 0.5) / sample_count as f32 - 0.5;
        let v = radical_inverse_vdc(index) - 0.5;
        let sample_position = light.center
            + light.axis_u * (u * light.width)
            + light.axis_v * (v * light.height);
        let to_light = sample_position - position;
        let distance2 = to_light.length_squared().max(1.0e-5);
        let distance = distance2.sqrt();
        let l = to_light / distance;
        let receiver_cos = normal.dot(l).max(0.0);
        if receiver_cos <= 0.0 {
            continue;
        }
        let emitter_dot = light.normal.dot(-l);
        let emitter_cos = if light.two_sided {
            emitter_dot.abs()
        } else {
            emitter_dot.max(0.0)
        };
        if emitter_cos <= 0.0 {
            continue;
        }
        let max_distance = (distance - bias.max(0.0001) * 2.0).max(0.0);
        if trace_any(origin, l, max_distance, ignore_triangle, triangles) {
            continue;
        }
        irradiance += emitted * receiver_cos * emitter_cos * area / distance2;
    }

    irradiance / sample_count as f32
}
