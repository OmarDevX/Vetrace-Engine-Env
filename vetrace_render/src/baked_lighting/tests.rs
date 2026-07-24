use super::*;
use crate::resources::BakedProbeGrid;

fn minimal_v5_file() -> BakedLightingFile {
    BakedLightingFile {
        version: BAKED_LIGHTING_FILE_VERSION,
        source_name: "test".to_string(),
        atlas_width: 4,
        atlas_height: 8,
        atlas_rgba16f: vec![0; 4 * 8 * 4],
        lightmaps: vec![(
            1,
            BakedLightmapRegion {
                uv_scale_offset: glam::Vec4::new(0.5, 0.25, 0.0, 0.0),
                intensity: 1.0,
                static_lighting_only: true,
                preserve_local_lights: false,
            },
        )],
        probes: BakedProbeGrid {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            counts: [1, 1, 1],
            samples: vec![BakedProbeSample::default()],
        },
        indirect_probes: BakedProbeGrid {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            counts: [1, 1, 1],
            samples: vec![BakedProbeSample::default()],
        },
    }
}

#[test]
fn version_five_atlas_requires_regions_in_combined_top_half() {
    let mut file = minimal_v5_file();
    assert!(validate_file(&file).is_ok());
    file.lightmaps[0].1.uv_scale_offset.w = 0.5;
    assert!(validate_file(&file).is_err());
}

#[test]
fn probe_grid_trilinear_sampling_is_stable() {
    let mut sample = BakedProbeSample::default();
    sample.sh_coefficients[0] = Vec3::splat(2.0);
    let grid = BakedProbeGrid {
        min: Vec3::ZERO,
        max: Vec3::ONE,
        counts: [2, 2, 2],
        samples: vec![sample; 8],
    };
    assert!(
        (grid.sample(Vec3::splat(0.37)).sh_coefficients[0] - Vec3::splat(2.0)).length() < 1.0e-5
    );
}
