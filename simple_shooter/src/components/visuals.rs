use glam::Vec3;
use vetrace_core::Actor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerVisualKind {
    BodyOutline,
    NameLabel,
    FirstPersonWeapon,
    WorldWeapon,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerVisualOwner {
    pub owner: Actor,
    pub kind: PlayerVisualKind,
}

#[derive(Clone, Copy, Debug)]
pub struct ShooterOutlineStyle {
    /// Game-side outline policy. Simple Shooter owns the shell entity and only
    /// hands the renderer a normal custom-shader material.
    pub local_enabled: bool,
    pub remote_enabled: bool,
    pub color: Vec3,
    pub thickness: f32,
}

impl Default for ShooterOutlineStyle {
    fn default() -> Self {
        Self {
            local_enabled: false,
            remote_enabled: true,
            color: Vec3::ZERO,
            thickness: 0.10,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShooterOutlineShell;

#[derive(Clone, Copy, Debug)]
pub struct ShooterOutlineOwner(pub Actor);

#[derive(Clone, Copy, Debug)]
pub struct PlayerGradientShader {
    /// Stable seed sent over the network. This remains the name/client seed so
    /// snapshots stay compact and deterministic. The final visual seed also
    /// mixes in the authoritative player id so two clients named `Player` do
    /// not collapse to the same gradient.
    pub color_seed: u64,
    pub visual_seed: u64,
    pub seed: f32,
    pub time: f32,
    pub color_a: Vec3,
    pub color_b: Vec3,
}

const EXPLICIT_PLAYER_COLOR_FLAG: u64 = 1_u64 << 63;

/// Encodes a user-selected visual seed. Explicit selections must render the
/// same palette in previews, local gameplay, and remote snapshots regardless
/// of the authoritative network player id assigned later.
pub fn explicit_player_color_seed(seed: u64) -> u64 {
    EXPLICIT_PLAYER_COLOR_FLAG | (seed & !EXPLICIT_PLAYER_COLOR_FLAG).max(1)
}

pub fn automatic_player_color_seed(seed: u64) -> u64 {
    (seed & !EXPLICIT_PLAYER_COLOR_FLAG).max(1)
}

impl PlayerGradientShader {
    pub fn new(player_id: u64, color_seed: u64) -> Self {
        let visual_seed = player_visual_seed(player_id, color_seed);
        let seedf = compact_shader_seed(visual_seed);
        let (color_a, color_b) = seeded_gradient_pair(visual_seed);
        Self {
            color_seed,
            visual_seed,
            seed: seedf,
            time: 0.0,
            color_a,
            color_b,
        }
    }

    pub fn sample(&self, y: f32, health01: f32) -> Vec3 {
        let t = 0.5 + 0.5 * (y * 2.5 + self.time * 3.0 + self.seed).sin();
        self.color_a.lerp(self.color_b, t).clamp(Vec3::splat(0.05), Vec3::ONE) * (0.35 + health01.clamp(0.0, 1.0) * 0.65)
    }
}

fn player_visual_seed(player_id: u64, color_seed: u64) -> u64 {
    if color_seed & EXPLICIT_PLAYER_COLOR_FLAG != 0 {
        return (color_seed & !EXPLICIT_PLAYER_COLOR_FLAG).max(1);
    }
    // Mix the stable network seed with the authoritative player id. This makes
    // same-name multiplayer clients visually different while keeping colors
    // deterministic across host and all clients.
    let mut x = color_seed ^ player_id.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

fn compact_shader_seed(seed: u64) -> f32 {
    // Keep WGSL/sin math in a small deterministic range.
    let compact = ((seed >> 16) ^ seed) & 0xffff;
    1.0 + compact as f32 * 0.013_37
}

fn seeded_gradient_pair(seed: u64) -> (Vec3, Vec3) {
    // Palette-style HSV generation gives each player an obviously different
    // gradient. The previous sine-channel method could produce visually close
    // or gray pairs, especially with similar/default player names.
    let hue = unit_from_bits(seed);
    let hue_b = (hue + 0.28 + unit_from_bits(seed.rotate_left(17)) * 0.18).fract();
    let sat_a = 0.72 + unit_from_bits(seed.rotate_left(29)) * 0.24;
    let sat_b = 0.68 + unit_from_bits(seed.rotate_left(41)) * 0.28;
    let val_a = 0.82 + unit_from_bits(seed.rotate_left(7)) * 0.16;
    let val_b = 0.78 + unit_from_bits(seed.rotate_left(53)) * 0.20;
    (hsv_to_rgb(hue, sat_a, val_a), hsv_to_rgb(hue_b, sat_b, val_b))
}

fn unit_from_bits(seed: u64) -> f32 {
    let bits = ((seed >> 40) as u32) ^ (seed as u32);
    (bits as f32 / u32::MAX as f32).clamp(0.0, 1.0)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let h = h.fract() * 6.0;
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i as i32 {
        0 => Vec3::new(v, t, p),
        1 => Vec3::new(q, v, p),
        2 => Vec3::new(p, v, t),
        3 => Vec3::new(p, q, v),
        4 => Vec3::new(t, p, v),
        _ => Vec3::new(v, p, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_color_seed_is_stable_across_player_ids() {
        let selected = explicit_player_color_seed(42);
        let preview = PlayerGradientShader::new(0x4d45_4e55, selected);
        let offline_player = PlayerGradientShader::new(1, selected);
        let network_player = PlayerGradientShader::new(9182, selected);
        assert_eq!(preview.visual_seed, offline_player.visual_seed);
        assert_eq!(preview.visual_seed, network_player.visual_seed);
        assert_eq!(preview.color_a, network_player.color_a);
        assert_eq!(preview.color_b, network_player.color_b);
    }

    #[test]
    fn ordinary_seed_still_distinguishes_player_ids() {
        assert_ne!(player_visual_seed(1, 42), player_visual_seed(2, 42));
    }
}
