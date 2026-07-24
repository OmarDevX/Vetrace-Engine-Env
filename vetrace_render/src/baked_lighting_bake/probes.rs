use super::*;

// Probe-grid generation, L2 SH projection, and visibility data.

pub(super) fn bake_probe_grid(
    triangles: &[BakeTriangle],
    directional: &[RenderDirectionalLight],
    points: &[RenderPointLight],
    spots: &[RenderSpotLight],
    areas: &[BakeRectAreaLight],
    bounds_min: Vec3,
    bounds_max: Vec3,
    config: &BakedLightingBakeConfig,
) -> BakedProbeGrid {
    let padding = Vec3::splat(config.probe_bounds_padding.max(0.01));
    let min = bounds_min - padding;
    let max = bounds_max + padding;
    let counts = config.probe_counts.map(|count| count.max(1));
    let bounce_count = config.indirect_bounces.max(1);
    let visibility_max_distance = (max - min).length().max(0.5);
    let mut previous: Option<BakedProbeGrid> = None;

    for _bounce in 0..bounce_count {
        let mut samples = Vec::with_capacity((counts[0] * counts[1] * counts[2]) as usize);
        for z in 0..counts[2] {
            for y in 0..counts[1] {
                for x in 0..counts[0] {
                    let t = Vec3::new(
                        grid_fraction(x, counts[0]),
                        grid_fraction(y, counts[1]),
                        grid_fraction(z, counts[2]),
                    );
                    let position = min + (max - min) * t;
                    samples.push(bake_probe(
                        position,
                        triangles,
                        directional,
                        points,
                        spots,
                        areas,
                        previous.as_ref(),
                        config,
                        visibility_max_distance,
                    ));
                }
            }
        }
        previous = Some(BakedProbeGrid { min, max, counts, samples });
    }

    previous.expect("indirect_bounces is clamped to at least one")
}

pub(super) fn bake_probe(
    position: Vec3,
    triangles: &[BakeTriangle],
    directional: &[RenderDirectionalLight],
    points: &[RenderPointLight],
    spots: &[RenderSpotLight],
    areas: &[BakeRectAreaLight],
    previous_bounce: Option<&BakedProbeGrid>,
    config: &BakedLightingBakeConfig,
    visibility_max_distance: f32,
) -> BakedProbeSample {
    let rays = config.probe_rays.max(16);
    let mut sh_coefficients = [Vec3::ZERO; 9];
    for index in 0..rays {
        let direction = fibonacci_direction(index, rays);
        let radiance = if let Some(hit) = trace_nearest(position, direction, f32::INFINITY, None, triangles) {
            let triangle = triangles[hit.triangle_index];
            let direct = direct_irradiance(
                hit.position,
                hit.normal,
                Some(hit.triangle_index),
                triangles,
                directional,
                points,
                spots,
                areas,
                config.surface_bias,
            );
            let previous_indirect = previous_bounce
                .map(|grid| grid.sample(hit.position).irradiance_for_normal(hit.normal))
                .unwrap_or(Vec3::ZERO)
                * config.indirect_bounce_decay.max(0.0);
            triangle.emissive
                + triangle.albedo
                    * (direct + previous_indirect + config.environment_radiance * 0.15)
                    / PI
        } else {
            config.environment_radiance.max(Vec3::ZERO)
        };
        let basis = sh_basis(direction);
        let solid_angle = 4.0 * PI / rays as f32;
        for coefficient in 0..sh_coefficients.len() {
            sh_coefficients[coefficient] += radiance * basis[coefficient] * solid_angle;
        }
    }
    apply_lambertian_convolution(&mut sh_coefficients);
    BakedProbeSample {
        sh_coefficients,
        visibility: bake_probe_visibility(position, triangles, config.surface_bias, visibility_max_distance),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn add_direct_lighting_to_probe_grid(
    indirect: &BakedProbeGrid,
    triangles: &[BakeTriangle],
    directional: &[RenderDirectionalLight],
    points: &[RenderPointLight],
    spots: &[RenderSpotLight],
    areas: &[BakeRectAreaLight],
    config: &BakedLightingBakeConfig,
) -> BakedProbeGrid {
    let mut combined = indirect.clone();
    let extent = combined.max - combined.min;

    for z in 0..combined.counts[2] {
        for y in 0..combined.counts[1] {
            for x in 0..combined.counts[0] {
                let t = Vec3::new(
                    grid_fraction(x, combined.counts[0]),
                    grid_fraction(y, combined.counts[1]),
                    grid_fraction(z, combined.counts[2]),
                );
                let position = combined.min + extent * t;
                let direct_sh = project_direct_irradiance_to_sh(
                    position,
                    triangles,
                    directional,
                    points,
                    spots,
                    areas,
                    config,
                );
                let index = (z * combined.counts[1] * combined.counts[0]
                    + y * combined.counts[0]
                    + x) as usize;
                let sample = &mut combined.samples[index];
                for coefficient in 0..sample.sh_coefficients.len() {
                    sample.sh_coefficients[coefficient] += direct_sh[coefficient];
                }
            }
        }
    }

    combined
}

pub(super) fn project_direct_irradiance_to_sh(
    position: Vec3,
    triangles: &[BakeTriangle],
    directional: &[RenderDirectionalLight],
    points: &[RenderPointLight],
    spots: &[RenderSpotLight],
    areas: &[BakeRectAreaLight],
    config: &BakedLightingBakeConfig,
) -> [Vec3; 9] {
    let ray_count = config.probe_rays.max(48).min(512);
    let mut sh_coefficients = [Vec3::ZERO; 9];
    let solid_angle = 4.0 * PI / ray_count as f32;
    for index in 0..ray_count {
        let normal = fibonacci_direction(index, ray_count);
        let irradiance = direct_irradiance(
            position,
            normal,
            None,
            triangles,
            directional,
            points,
            spots,
            areas,
            config.surface_bias,
        );
        let basis = sh_basis(normal);
        for coefficient in 0..sh_coefficients.len() {
            sh_coefficients[coefficient] += irradiance * basis[coefficient] * solid_angle;
        }
    }
    sh_coefficients
}

pub(super) fn sh_basis(direction: Vec3) -> [f32; 9] {
    let d = direction.normalize_or_zero();
    let x = d.x;
    let y = d.y;
    let z = d.z;
    [
        0.282095,
        0.488603 * y,
        0.488603 * z,
        0.488603 * x,
        1.092548 * x * y,
        1.092548 * y * z,
        0.315392 * (3.0 * z * z - 1.0),
        1.092548 * x * z,
        0.546274 * (x * x - y * y),
    ]
}

pub(super) fn apply_lambertian_convolution(coefficients: &mut [Vec3; 9]) {
    coefficients[0] *= PI;
    for coefficient in &mut coefficients[1..4] {
        *coefficient *= 2.0 * PI / 3.0;
    }
    for coefficient in &mut coefficients[4..9] {
        *coefficient *= PI / 4.0;
    }
}

pub(super) fn bake_probe_visibility(
    position: Vec3,
    triangles: &[BakeTriangle],
    bias: f32,
    max_distance: f32,
) -> [f32; 6] {
    let directions = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];
    let mut out = [max_distance.max(0.25); 6];
    for (index, direction) in directions.iter().enumerate() {
        let origin = position + *direction * bias.max(0.0005);
        if let Some(hit) = trace_nearest(origin, *direction, max_distance, None, triangles) {
            out[index] = (hit.position - position).length().max(0.02);
        }
    }
    out
}
