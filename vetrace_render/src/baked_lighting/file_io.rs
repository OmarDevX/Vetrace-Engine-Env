// `.vlight` loading, validation, saving, and installation.

use super::*;

const BAKED_LIGHTING_MAGIC: &[u8; 8] = b"VLIGHT02";
const LEGACY_BAKED_LIGHTING_MAGIC: &[u8; 8] = b"VLIGHT01";
const MAX_BAKED_LIGHTING_FILE_BYTES: u64 = 768 * 1024 * 1024;

/// Loads baked lighting and installs its immutable RGBA16F atlas.
/// This function never bakes and returns an error for missing/incompatible data.
pub fn load_baked_lighting(engine: &mut Engine, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    let metadata = fs::metadata(path.as_ref())?;
    if metadata.len() > MAX_BAKED_LIGHTING_FILE_BYTES {
        return Err("baked-lighting file exceeds the 768 MiB safety limit".into());
    }
    let bytes = fs::read(path.as_ref())?;
    if bytes.starts_with(LEGACY_BAKED_LIGHTING_MAGIC) {
        return Err("legacy RGBM8 baked-lighting file detected; rebake it as version 5 RGBA16F".into());
    }
    if !bytes.starts_with(BAKED_LIGHTING_MAGIC) {
        return Err("invalid baked-lighting file header".into());
    }
    let file: BakedLightingFile = bincode::deserialize(&bytes[BAKED_LIGHTING_MAGIC.len()..])?;
    validate_file(&file)?;
    install_baked_lighting(engine, file);
    Ok(())
}

/// Explicitly performs a CPU bake, saves it, and immediately installs the
/// baked result. Call this from tools or a dedicated `--bake-lighting` mode,
/// not from the normal gameplay loop.
pub fn bake_and_save_baked_lighting(
    engine: &mut Engine,
    path: impl AsRef<Path>,
    config: &BakedLightingBakeConfig,
) -> Result<BakedLightingBakeReport, Box<dyn Error>> {
    let (file, mut report) = bake_baked_lighting(engine, config)?;
    validate_file(&file)?;
    if let Some(parent) = path.as_ref().parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let payload = bincode::serialize(&file)?;
    let mut bytes = Vec::with_capacity(BAKED_LIGHTING_MAGIC.len() + payload.len());
    bytes.extend_from_slice(BAKED_LIGHTING_MAGIC);
    bytes.extend_from_slice(&payload);
    fs::write(path.as_ref(), &bytes)?;
    report.output_bytes = bytes.len() as u64;

    install_baked_lighting(engine, file);
    Ok(report)
}

fn install_baked_lighting(engine: &mut Engine, file: BakedLightingFile) {
    unload_baked_lighting(engine);
    let atlas = std::sync::Arc::new(BakedLightmapAtlas::new(
        file.atlas_width,
        file.atlas_height,
        file.atlas_rgba16f,
    ));
    engine.insert_resource(BakedLightingScene {
        enabled: true,
        debug_mode: BakedLightingDebugMode::Off,
        runtime_mode: BakedLightingRuntimeMode::BakedOnly,
        source_name: file.source_name,
        atlas: Some(atlas),
        atlas_width: file.atlas_width,
        atlas_height: file.atlas_height,
        lightmaps: file.lightmaps.into_iter().collect(),
        probes: file.probes,
        indirect_probes: file.indirect_probes,
    });
}

pub(super) fn validate_file(file: &BakedLightingFile) -> Result<(), Box<dyn Error>> {
    if file.version != BAKED_LIGHTING_FILE_VERSION {
        return Err(format!(
            "unsupported baked-lighting version {}; expected {} (rebake the .vlight file)",
            file.version, BAKED_LIGHTING_FILE_VERSION
        )
        .into());
    }
    const MAX_ATLAS_DIMENSION: u32 = 8192;
    if file.atlas_width == 0
        || file.atlas_height == 0
        || file.atlas_width > MAX_ATLAS_DIMENSION
        || file.atlas_height > MAX_ATLAS_DIMENSION
        || file.atlas_height % 2 != 0
    {
        return Err("invalid baked-lighting atlas dimensions".into());
    }
    let expected = (file.atlas_width as usize)
        .checked_mul(file.atlas_height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("baked-lighting atlas size overflow")?;
    if file.atlas_rgba16f.len() != expected {
        return Err("invalid baked-lighting RGBA16F atlas pixel count".into());
    }
    if file.atlas_rgba16f.iter().any(|value| {
        let exponent = value & 0x7c00;
        let negative = value & 0x8000 != 0;
        negative || exponent == 0x7c00
    }) {
        return Err("baked-lighting RGBA16F atlas contains a negative, NaN, or infinite value".into());
    }
    if file.source_name.trim().is_empty() || file.source_name.len() > 4096 {
        return Err("invalid baked-lighting source name".into());
    }
    let mut object_keys = HashSet::with_capacity(file.lightmaps.len());
    for (key, region) in &file.lightmaps {
        let [scale_x, scale_y, offset_x, offset_y] = region.uv_scale_offset.to_array();
        let transform_is_valid = region.uv_scale_offset.is_finite()
            && scale_x > 0.0
            && scale_y > 0.0
            && scale_x <= 1.0
            && scale_y <= 1.0
            && offset_x >= 0.0
            && offset_y >= 0.0
            && offset_x + scale_x <= 1.0001
            && offset_y + scale_y <= 0.5001;
        if !object_keys.insert(*key)
            || !transform_is_valid
            || !region.intensity.is_finite()
            || region.intensity < 0.0
        {
            return Err("invalid or duplicate baked-lighting lightmap region".into());
        }
    }
    if !file.probes.is_valid() || !file.indirect_probes.is_valid() {
        return Err("invalid baked-lighting probe grid".into());
    }
    if file.probes.counts != file.indirect_probes.counts
        || file.probes.min != file.indirect_probes.min
        || file.probes.max != file.indirect_probes.max
    {
        return Err("combined and indirect probe grids use different layouts".into());
    }
    Ok(())
}
