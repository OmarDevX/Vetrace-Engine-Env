use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};

pub const BAKED_LIGHTING_FILE_VERSION: u32 = 5;

/// Runtime policy for combining baked diffuse lighting with realtime direct light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BakedLightingRuntimeMode {
    /// Sample the combined direct + indirect lightmap and suppress duplicate
    /// realtime directional/ambient lighting on static receivers.
    #[default]
    BakedOnly,
    /// Sample the indirect-only half of the atlas and keep realtime direct
    /// lighting/shadows. This is intended for high-quality hybrid rendering.
    HybridRealtimeDirect,
}

impl BakedLightingRuntimeMode {
    pub(crate) fn shader_value(self) -> f32 {
        match self {
            Self::BakedOnly => 0.0,
            Self::HybridRealtimeDirect => 1.0,
        }
    }
}

/// Runtime visualization used to verify baked data without changing the bake.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BakedLightingDebugMode {
    #[default]
    Off,
    /// Show decoded baked irradiance on static lightmapped geometry.
    Lightmap,
    /// Show authored UV2 coordinates and chart continuity.
    LightmapUv,
    /// Show directional probe irradiance on probe receivers.
    Probes,
}

impl BakedLightingDebugMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Lightmap,
            Self::Lightmap => Self::LightmapUv,
            Self::LightmapUv => Self::Probes,
            Self::Probes => Self::Off,
        }
    }

    pub(crate) fn shader_value(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Lightmap => 1.0,
            Self::LightmapUv => 2.0,
            Self::Probes => 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct BakedLightmapRegion {
    /// xy = atlas scale, zw = atlas offset.
    pub uv_scale_offset: Vec4,
    pub intensity: f32,
    pub static_lighting_only: bool,
    pub preserve_local_lights: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct BakedProbeSample {
    /// Irradiance SH coefficients (L2 / 9 coefficients) already convolved with
    /// the Lambertian cosine kernel, so runtime evaluation is just basis dot
    /// coefficient. This is smoother and more expressive than the old six-axis
    /// lobe approximation while staying cheap at runtime.
    pub sh_coefficients: [Vec3; 9],
    /// Approximate free-space distance along +/-X, +/-Y, +/-Z from the probe
    /// center. Runtime trilinear sampling uses this to reduce cross-wall probe
    /// leakage without any per-frame ray tracing.
    pub visibility: [f32; 6],
}

impl BakedProbeSample {
    pub fn add_scaled(&mut self, other: Self, weight: f32) {
        for index in 0..self.sh_coefficients.len() {
            self.sh_coefficients[index] += other.sh_coefficients[index] * weight;
        }
        for index in 0..self.visibility.len() {
            self.visibility[index] += other.visibility[index] * weight;
        }
    }

    pub fn scale(&mut self, factor: f32) {
        for coefficient in &mut self.sh_coefficients {
            *coefficient *= factor;
        }
        for distance in &mut self.visibility {
            *distance *= factor;
        }
    }

    pub fn is_finite(&self) -> bool {
        self.sh_coefficients.iter().all(|value| value.is_finite())
            && self.visibility.iter().all(|value| value.is_finite() && *value >= 0.0)
    }

    pub fn irradiance_for_normal(&self, normal: Vec3) -> Vec3 {
        let n = normal.normalize_or_zero();
        let basis = sh_basis(n);
        let mut out = Vec3::ZERO;
        for (coefficient, basis_value) in self.sh_coefficients.iter().zip(basis) {
            out += *coefficient * basis_value;
        }
        out.max(Vec3::ZERO)
    }

    fn visibility_distance_for_direction(&self, direction: Vec3) -> f32 {
        let dir = direction.normalize_or_zero();
        if dir.length_squared() <= 1.0e-8 {
            return self.visibility.iter().copied().fold(0.0, f32::max);
        }
        let weights = dir.abs();
        let sum = (weights.x + weights.y + weights.z).max(1.0e-5);
        let x = if dir.x >= 0.0 { self.visibility[0] } else { self.visibility[1] } * weights.x;
        let y = if dir.y >= 0.0 { self.visibility[2] } else { self.visibility[3] } * weights.y;
        let z = if dir.z >= 0.0 { self.visibility[4] } else { self.visibility[5] } * weights.z;
        (x + y + z) / sum
    }

    pub fn visibility_weight(&self, receiver_position: Vec3, probe_position: Vec3) -> f32 {
        let max_visibility = self.visibility.iter().copied().fold(0.0, f32::max);
        if max_visibility <= 1.0e-5 {
            return 1.0;
        }
        let offset = receiver_position - probe_position;
        let distance = offset.length();
        if distance <= 1.0e-4 {
            return 1.0;
        }
        let expected = self.visibility_distance_for_direction(offset).max(0.03);
        let safe_distance = expected * 1.10 + 0.04;
        if distance <= safe_distance {
            return 1.0;
        }
        let fade_range = (expected * 0.45).max(0.08);
        let t = ((distance - safe_distance) / fade_range).clamp(0.0, 1.0);
        (1.0 - t * t).max(0.0)
    }
}

fn sh_basis(direction: Vec3) -> [f32; 9] {
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BakedProbeGrid {

    pub min: Vec3,
    pub max: Vec3,
    pub counts: [u32; 3],
    pub samples: Vec<BakedProbeSample>,
}

impl BakedProbeGrid {
    const MAX_AXIS_PROBES: u32 = 256;
    const MAX_TOTAL_PROBES: usize = 262_144;

    fn expected_sample_count(&self) -> Option<usize> {
        if self
            .counts
            .iter()
            .any(|count| *count == 0 || *count > Self::MAX_AXIS_PROBES)
            || !self.min.is_finite()
            || !self.max.is_finite()
            || !self.max.cmpgt(self.min).all()
        {
            return None;
        }
        let expected = (self.counts[0] as usize)
            .checked_mul(self.counts[1] as usize)
            .and_then(|xy| xy.checked_mul(self.counts[2] as usize))?;
        (expected <= Self::MAX_TOTAL_PROBES).then_some(expected)
    }

    fn has_valid_layout(&self) -> bool {
        self.expected_sample_count() == Some(self.samples.len())
    }

    pub fn is_valid(&self) -> bool {
        self.has_valid_layout() && self.samples.iter().all(BakedProbeSample::is_finite)
    }

    pub fn sample(&self, position: Vec3) -> BakedProbeSample {
        // Full coefficient/visibility validation is done once when a bake file
        // is loaded or generated. Keep the per-object runtime lookup constant
        // time instead of rescanning the entire probe volume every frame.
        if !self.has_valid_layout() {
            return BakedProbeSample::default();
        }
        let extent = (self.max - self.min).max(Vec3::splat(0.0001));
        let normalized = ((position - self.min) / extent).clamp(Vec3::ZERO, Vec3::ONE);
        let fx = normalized.x * (self.counts[0] - 1) as f32;
        let fy = normalized.y * (self.counts[1] - 1) as f32;
        let fz = normalized.z * (self.counts[2] - 1) as f32;
        let x0 = fx.floor() as u32;
        let y0 = fy.floor() as u32;
        let z0 = fz.floor() as u32;
        let x1 = (x0 + 1).min(self.counts[0] - 1);
        let y1 = (y0 + 1).min(self.counts[1] - 1);
        let z1 = (z0 + 1).min(self.counts[2] - 1);
        let tx = fx.fract();
        let ty = fy.fract();
        let tz = fz.fract();

        let mut weighted = BakedProbeSample::default();
        let mut weighted_total = 0.0_f32;
        let mut fallback = BakedProbeSample::default();
        let mut fallback_total = 0.0_f32;
        for &(x, wx) in &[(x0, 1.0 - tx), (x1, tx)] {
            for &(y, wy) in &[(y0, 1.0 - ty), (y1, ty)] {
                for &(z, wz) in &[(z0, 1.0 - tz), (z1, tz)] {
                    let base_weight = wx * wy * wz;
                    if base_weight <= 0.0 {
                        continue;
                    }
                    let index = self.index(x, y, z);
                    let sample = self.samples[index];
                    fallback.add_scaled(sample, base_weight);
                    fallback_total += base_weight;
                    let probe_position = self.position_for_index(x, y, z);
                    let visibility = sample.visibility_weight(position, probe_position);
                    let weight = base_weight * visibility;
                    if weight > 1.0e-5 {
                        weighted.add_scaled(sample, weight);
                        weighted_total += weight;
                    }
                }
            }
        }
        if weighted_total > 1.0e-5 {
            weighted.scale(1.0 / weighted_total);
            weighted
        } else if fallback_total > 1.0e-5 {
            fallback.scale(1.0 / fallback_total);
            fallback
        } else {
            BakedProbeSample::default()
        }
    }

    fn position_for_index(&self, x: u32, y: u32, z: u32) -> Vec3 {
        let extent = self.max - self.min;
        let tx = if self.counts[0] <= 1 { 0.5 } else { x as f32 / (self.counts[0] - 1) as f32 };
        let ty = if self.counts[1] <= 1 { 0.5 } else { y as f32 / (self.counts[1] - 1) as f32 };
        let tz = if self.counts[2] <= 1 { 0.5 } else { z as f32 / (self.counts[2] - 1) as f32 };
        self.min + extent * Vec3::new(tx, ty, tz)
    }

    fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (z * self.counts[1] * self.counts[0] + y * self.counts[0] + x) as usize
    }
}



static NEXT_BAKED_LIGHTMAP_ATLAS_ID: AtomicU64 = AtomicU64::new(1);

/// CPU-side immutable RGBA16F atlas shared once per render frame.
///
/// The renderer uploads this to a filterable `Rgba16Float` GPU texture. Keeping
/// it separate from ordinary material textures avoids changing the public
/// `TextureAsset` format just for baked HDR lighting.
#[derive(Debug)]
pub(crate) struct BakedLightmapAtlas {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub rgba16f: Vec<u16>,
}

impl BakedLightmapAtlas {
    pub(crate) fn new(width: u32, height: u32, rgba16f: Vec<u16>) -> Self {
        Self {
            id: NEXT_BAKED_LIGHTMAP_ATLAS_ID.fetch_add(1, Ordering::Relaxed),
            width,
            height,
            rgba16f,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BakedLightingScene {
    pub enabled: bool,
    pub debug_mode: BakedLightingDebugMode,
    pub runtime_mode: BakedLightingRuntimeMode,
    pub source_name: String,
    pub(crate) atlas: Option<Arc<BakedLightmapAtlas>>,
    pub atlas_width: u32,
    /// Physical atlas height. The top half stores combined lighting and the
    /// bottom half stores indirect-only lighting.
    pub atlas_height: u32,
    pub lightmaps: HashMap<u64, BakedLightmapRegion>,
    /// Direct + indirect irradiance used by probe receivers in baked-only mode.
    pub probes: BakedProbeGrid,
    /// Indirect-only irradiance used when realtime direct lighting is enabled.
    pub indirect_probes: BakedProbeGrid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct BakedLightingFile {
    pub version: u32,
    pub source_name: String,
    pub atlas_width: u32,
    /// Physical atlas height. The top half stores combined lighting and the
    /// bottom half stores indirect-only lighting.
    pub atlas_height: u32,
    /// Raw IEEE-754 binary16 RGBA texels. RGB stores linear HDR irradiance and
    /// alpha is reserved as 1.0.
    pub atlas_rgba16f: Vec<u16>,
    pub lightmaps: Vec<(u64, BakedLightmapRegion)>,
    pub probes: BakedProbeGrid,
    pub indirect_probes: BakedProbeGrid,
}
