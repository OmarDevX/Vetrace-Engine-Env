use std::collections::BTreeMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use vetrace_core::Actor;

pub const DEFAULT_WEAPON_ID: &str = "vetrace_rifle";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeaponPresentation {
    FirstPerson,
    #[default]
    World,
}

#[derive(Clone, Debug)]
pub struct WeaponAttachment {
    pub owner: Actor,
    pub weapon_id: String,
    pub presentation: WeaponPresentation,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WeaponPart;

#[derive(Clone, Copy, Debug)]
pub struct MuzzleFlash {
    pub ttl: f32,
}

#[derive(Clone, Debug)]
pub struct EquippedWeapon {
    pub weapon_id: String,
    pub cooldown_remaining: f32,
}

impl Default for EquippedWeapon {
    fn default() -> Self {
        Self { weapon_id: DEFAULT_WEAPON_ID.to_string(), cooldown_remaining: 0.0 }
    }
}

#[derive(Clone, Debug)]
pub struct FireRequest {
    pub shooter: Actor,
    pub shooter_id: u64,
    pub weapon_id: String,
    pub aim_origin: Vec3,
    pub aim_direction: Vec3,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShotResult {
    pub shooter_id: u64,
    pub weapon_id: String,
    pub muzzle: Vec3,
    pub endpoint: Vec3,
    pub hit_id: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct ShooterPresentationConfig {
    pub enabled: bool,
}

impl Default for ShooterPresentationConfig {
    fn default() -> Self {
        Self { enabled: cfg!(any(feature = "window", feature = "gpu_render", feature = "software_window")) }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeaponAimMode {
    /// The camera chooses the target and the physical shot converges on it from
    /// the muzzle. This gives normal FPS crosshair behavior without faking the
    /// visible tracer origin.
    CrosshairConverge,
    /// The shot follows the gun's local -Z axis exactly.
    BarrelForward,
}

impl Default for WeaponAimMode {
    fn default() -> Self { Self::CrosshairConverge }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponGameplay {
    pub damage: i32,
    pub cooldown_seconds: f32,
    pub range: f32,
    pub aim_mode: WeaponAimMode,
}

impl Default for WeaponGameplay {
    fn default() -> Self {
        Self { damage: 25, cooldown_seconds: 0.32, range: 60.0, aim_mode: WeaponAimMode::CrosshairConverge }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponAttachmentConfig {
    /// Gun-root offset in aim-local coordinates: +X right, +Y up, -Z forward.
    pub position: [f32; 3],
    /// View-model offset. This never changes the authoritative muzzle path.
    pub first_person_position: [f32; 3],
    /// Physical muzzle in gun-root local coordinates.
    pub muzzle: [f32; 3],
    pub rotation_degrees: [f32; 3],
}

impl Default for WeaponAttachmentConfig {
    fn default() -> Self {
        Self {
            position: [0.36, -0.24, -0.38],
            first_person_position: [0.38, -0.29, -0.50],
            muzzle: [0.0, 0.015, -0.78],
            rotation_degrees: [0.0; 3],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponModelConfig {
    /// Optional GLB/GLTF path relative to `simple_shooter/assets`. When absent,
    /// a lightweight procedural rifle is assembled from the sizes below.
    pub path: Option<String>,
    pub position: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub scale: [f32; 3],
    pub body_size: [f32; 3],
    pub barrel_size: [f32; 3],
    pub stock_size: [f32; 3],
    pub grip_size: [f32; 3],
    pub body_color: [f32; 3],
    pub accent_color: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
}

impl Default for WeaponModelConfig {
    fn default() -> Self {
        Self {
            path: None,
            position: [0.0; 3],
            rotation_degrees: [0.0; 3],
            scale: [1.0; 3],
            body_size: [0.18, 0.18, 0.52],
            barrel_size: [0.065, 0.065, 0.42],
            stock_size: [0.16, 0.14, 0.26],
            grip_size: [0.10, 0.25, 0.12],
            body_color: [0.075, 0.085, 0.10],
            accent_color: [0.24, 0.27, 0.31],
            roughness: 0.32,
            metallic: 0.75,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TracerConfig {
    pub enabled: bool,
    pub width: f32,
    pub lifetime_seconds: f32,
    pub color: [f32; 3],
    pub emissive: [f32; 3],
    pub light_intensity: f32,
    pub light_range: f32,
    pub light_samples: u8,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            width: 0.045,
            lifetime_seconds: 0.12,
            color: [1.0, 0.90, 0.25],
            emissive: [1.0, 0.55, 0.05],
            light_intensity: 1.6,
            light_range: 4.5,
            light_samples: 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct MuzzleFlashConfig {
    pub enabled: bool,
    pub lifetime_seconds: f32,
    pub size: f32,
    pub color: [f32; 3],
    pub emissive_intensity: f32,
    pub light_intensity: f32,
    pub light_range: f32,
}

impl Default for MuzzleFlashConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lifetime_seconds: 0.055,
            size: 0.18,
            color: [1.0, 0.48, 0.06],
            emissive_intensity: 5.0,
            light_intensity: 1.6,
            light_range: 5.5,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponSoundConfig {
    pub path: String,
    pub volume: f32,
    pub max_distance: f32,
}

impl Default for WeaponSoundConfig {
    fn default() -> Self { Self { path: "shoot.mp3".to_string(), volume: 0.95, max_distance: 70.0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponDefinition {
    pub id: String,
    pub name: String,
    pub gameplay: WeaponGameplay,
    pub attachment: WeaponAttachmentConfig,
    pub model: WeaponModelConfig,
    pub tracer: TracerConfig,
    pub muzzle_flash: MuzzleFlashConfig,
    pub sound: WeaponSoundConfig,
}

impl Default for WeaponDefinition {
    fn default() -> Self {
        Self {
            id: DEFAULT_WEAPON_ID.to_string(),
            name: "Vetrace Rifle".to_string(),
            gameplay: WeaponGameplay::default(),
            attachment: WeaponAttachmentConfig::default(),
            model: WeaponModelConfig::default(),
            tracer: TracerConfig::default(),
            muzzle_flash: MuzzleFlashConfig::default(),
            sound: WeaponSoundConfig::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WeaponRegistry {
    definitions: BTreeMap<String, WeaponDefinition>,
    gameplay_fingerprint: u64,
}

impl Default for WeaponRegistry {
    fn default() -> Self { Self::from(WeaponDefinition::default()) }
}

impl WeaponRegistry {
    pub fn from_definitions(definitions: impl IntoIterator<Item = WeaponDefinition>) -> Self {
        let mut entries = BTreeMap::new();
        for mut definition in definitions {
            definition = definition.sanitized();
            if definition.id.trim().is_empty() { continue; }
            entries.insert(definition.id.clone(), definition);
        }
        if entries.is_empty() {
            let fallback = WeaponDefinition::default();
            entries.insert(fallback.id.clone(), fallback);
        }
        let gameplay_fingerprint = gameplay_fingerprint(&entries);
        Self { definitions: entries, gameplay_fingerprint }
    }

    pub fn get(&self, id: &str) -> Option<&WeaponDefinition> { self.definitions.get(id) }
    pub fn get_or_default(&self, id: &str) -> &WeaponDefinition {
        self.get(id)
            .or_else(|| self.get(DEFAULT_WEAPON_ID))
            .or_else(|| self.definitions.values().next())
            .expect("weapon registry always contains a fallback")
    }
    pub fn gameplay_fingerprint(&self) -> u64 { self.gameplay_fingerprint }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &WeaponDefinition)> {
        self.definitions.iter().map(|(id, definition)| (id.as_str(), definition))
    }
}

impl From<WeaponDefinition> for WeaponRegistry {
    fn from(definition: WeaponDefinition) -> Self { Self::from_definitions([definition]) }
}

fn gameplay_fingerprint(definitions: &BTreeMap<String, WeaponDefinition>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for (id, definition) in definitions {
        hash_weapon_bytes(&mut hash, id.as_bytes());
        hash_weapon_bytes(&mut hash, &definition.gameplay.damage.to_le_bytes());
        hash_weapon_bytes(&mut hash, &definition.gameplay.cooldown_seconds.to_bits().to_le_bytes());
        hash_weapon_bytes(&mut hash, &definition.gameplay.range.to_bits().to_le_bytes());
        hash_weapon_bytes(&mut hash, &[match definition.gameplay.aim_mode {
            WeaponAimMode::CrosshairConverge => 0,
            WeaponAimMode::BarrelForward => 1,
        }]);
    }
    hash
}

fn hash_weapon_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= *byte as u64;
        *hash = (*hash).wrapping_mul(0x1000_0000_01b3);
    }
}

impl WeaponDefinition {
    pub fn sanitized(mut self) -> Self {
        self.gameplay.damage = self.gameplay.damage.max(0);
        self.gameplay.cooldown_seconds = self.gameplay.cooldown_seconds.max(0.01);
        self.gameplay.range = self.gameplay.range.max(0.1);
        self.tracer.width = self.tracer.width.max(0.001);
        self.tracer.lifetime_seconds = self.tracer.lifetime_seconds.max(0.001);
        self.tracer.light_intensity = self.tracer.light_intensity.max(0.0);
        self.tracer.light_range = self.tracer.light_range.max(0.0);
        self.tracer.light_samples = self.tracer.light_samples.clamp(1, 4);
        self.muzzle_flash.lifetime_seconds = self.muzzle_flash.lifetime_seconds.max(0.001);
        self.muzzle_flash.size = self.muzzle_flash.size.max(0.001);
        self.muzzle_flash.light_intensity = self.muzzle_flash.light_intensity.max(0.0);
        self.muzzle_flash.light_range = self.muzzle_flash.light_range.max(0.0);
        self.sound.volume = self.sound.volume.max(0.0);
        self.sound.max_distance = self.sound.max_distance.max(0.1);
        self
    }
}

pub(crate) fn vec3(value: [f32; 3]) -> Vec3 { Vec3::from_array(value) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_changes_do_not_change_gameplay_fingerprint() {
        let original = WeaponDefinition::default();
        let mut visual_variant = original.clone();
        visual_variant.model.body_color = [1.0, 0.0, 1.0];
        visual_variant.tracer.width = 0.5;
        visual_variant.tracer.light_intensity = 9.0;
        visual_variant.muzzle_flash.light_range = 20.0;
        assert_eq!(WeaponRegistry::from(original).gameplay_fingerprint(), WeaponRegistry::from(visual_variant).gameplay_fingerprint());
    }

    #[test]
    fn damage_changes_gameplay_fingerprint() {
        let original = WeaponDefinition::default();
        let mut gameplay_variant = original.clone();
        gameplay_variant.gameplay.damage += 1;
        assert_ne!(WeaponRegistry::from(original).gameplay_fingerprint(), WeaponRegistry::from(gameplay_variant).gameplay_fingerprint());
    }

    #[test]
    fn unsafe_definition_values_are_sanitized() {
        let mut definition = WeaponDefinition::default();
        definition.gameplay.cooldown_seconds = -1.0;
        definition.gameplay.range = 0.0;
        definition.tracer.width = 0.0;
        definition.tracer.light_intensity = -1.0;
        definition.tracer.light_range = -1.0;
        definition.tracer.light_samples = 0;
        definition.muzzle_flash.light_intensity = -1.0;
        definition.muzzle_flash.light_range = -1.0;
        let definition = definition.sanitized();
        assert!(definition.gameplay.cooldown_seconds > 0.0);
        assert!(definition.gameplay.range > 0.0);
        assert!(definition.tracer.width > 0.0);
        assert_eq!(definition.tracer.light_intensity, 0.0);
        assert_eq!(definition.tracer.light_range, 0.0);
        assert_eq!(definition.tracer.light_samples, 1);
        assert_eq!(definition.muzzle_flash.light_intensity, 0.0);
        assert_eq!(definition.muzzle_flash.light_range, 0.0);
    }
}
