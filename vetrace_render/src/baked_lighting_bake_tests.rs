use super::*;

#[test]
fn half_float_black_and_one_are_exact() {
    assert_eq!(f32_to_f16_bits(0.0), 0x0000);
    assert_eq!(f32_to_f16_bits(1.0), 0x3c00);
    assert_eq!(f16_bits_to_f32(0x0000), 0.0);
    assert_eq!(f16_bits_to_f32(0x3c00), 1.0);
}

#[test]
fn half_float_lightmap_round_trip_stays_precise() {
    for source in [0.0005_f32, 0.03, 0.18, 1.0, 7.25, 16.0] {
        let decoded = f16_bits_to_f32(f32_to_f16_bits(source));
        let relative_error = (decoded - source).abs() / source.max(0.001);
        assert!(relative_error < 0.001, "{source} became {decoded}");
    }
}

#[test]
fn half_float_preserves_smooth_lightmap_gradients() {
    let mut levels = (0..4096)
        .map(|index| f32_to_f16_bits(index as f32 * (16.0 / 4095.0)))
        .collect::<Vec<_>>();
    levels.dedup();
    assert!(levels.len() > 3000, "only {} distinct gradient levels", levels.len());
}

#[test]
fn default_bake_config_is_valid() {
    assert!(validate_bake_config(&BakedLightingBakeConfig::default()).is_ok());
}

#[test]
fn oversized_lightmap_filter_is_rejected() {
    let config = BakedLightingBakeConfig {
        lightmap_filter_radius: 9,
        ..BakedLightingBakeConfig::default()
    };
    assert!(validate_bake_config(&config).is_err());
}

#[test]
fn oversized_probe_grid_is_rejected() {
    let config = BakedLightingBakeConfig {
        probe_counts: [256, 256, 5],
        ..BakedLightingBakeConfig::default()
    };
    assert!(validate_bake_config(&config).is_err());
}

#[test]
fn world_space_texel_density_scales_large_receivers() {
    let triangle = BakeTriangle {
        positions: [Vec3::ZERO, Vec3::new(40.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 40.0)],
        normals: [Vec3::Y; 3],
        lightmap_uvs: Some([Vec2::ZERO, Vec2::X, Vec2::Y]),
        albedo: Vec3::ONE,
        emissive: Vec3::ZERO,
        object_key: 1,
    };
    let config = BakedLightingBakeConfig {
        lightmap_resolution: 64,
        lightmap_texels_per_unit: 6.0,
        ..BakedLightingBakeConfig::default()
    };
    assert_eq!(receiver_lightmap_resolution(&[triangle], &config, 1.0), 240);
}

#[test]
fn zero_texel_density_preserves_fixed_resolution() {
    let triangle = BakeTriangle {
        positions: [Vec3::ZERO, Vec3::new(40.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 40.0)],
        normals: [Vec3::Y; 3],
        lightmap_uvs: Some([Vec2::ZERO, Vec2::X, Vec2::Y]),
        albedo: Vec3::ONE,
        emissive: Vec3::ZERO,
        object_key: 1,
    };
    let config = BakedLightingBakeConfig {
        lightmap_resolution: 64,
        lightmap_texels_per_unit: 0.0,
        ..BakedLightingBakeConfig::default()
    };
    assert_eq!(receiver_lightmap_resolution(&[triangle], &config, 1.0), 64);
}

#[test]
fn zero_indirect_bounces_is_rejected() {
    let config = BakedLightingBakeConfig {
        indirect_bounces: 0,
        ..BakedLightingBakeConfig::default()
    };
    assert!(validate_bake_config(&config).is_err());
}

#[test]
fn unstable_indirect_bounce_decay_is_rejected() {
    let config = BakedLightingBakeConfig {
        indirect_bounce_decay: 1.0,
        ..BakedLightingBakeConfig::default()
    };
    assert!(validate_bake_config(&config).is_err());
}

fn downward_test_area_light() -> BakeRectAreaLight {
    BakeRectAreaLight {
        entity: vetrace_core::Entity(1),
        center: Vec3::new(0.0, 2.0, 0.0),
        axis_u: Vec3::X,
        axis_v: Vec3::Z,
        normal: Vec3::NEG_Y,
        width: 1.0,
        height: 1.0,
        color: Vec3::ONE,
        intensity: 10.0,
        samples: 16,
        two_sided: false,
    }
}

#[test]
fn rectangular_area_light_illuminates_front_side() {
    let light = downward_test_area_light();
    let irradiance = rect_area_light_irradiance(
        Vec3::ZERO,
        Vec3::Y,
        Vec3::new(0.0, 0.001, 0.0),
        None,
        &[],
        light,
        0.001,
    );
    assert!(irradiance.min_element() > 0.5);
}

#[test]
fn one_sided_rectangular_area_light_rejects_back_side() {
    let mut light = downward_test_area_light();
    light.normal = Vec3::Y;
    let irradiance = rect_area_light_irradiance(
        Vec3::ZERO,
        Vec3::Y,
        Vec3::new(0.0, 0.001, 0.0),
        None,
        &[],
        light,
        0.001,
    );
    assert_eq!(irradiance, Vec3::ZERO);
}
