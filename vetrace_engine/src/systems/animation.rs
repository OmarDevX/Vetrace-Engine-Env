use std::sync::Arc;

use crate::{
    assets::{AssetManager, AnimationChannel},
    components::components::{Animation, Transform, MorphWeights},
    engine::engine::Engine,
    Behaviour,
};

/// System that advances `Animation` components and applies translation, rotation, and scale keyframes
/// to associated `Transform` components.
pub struct AnimationSystem {
    pub assets: Arc<AssetManager>,
}

impl AnimationSystem {
    pub fn new(assets: Arc<AssetManager>) -> Self {
        Self { assets }
    }
}

impl Behaviour for AnimationSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        // Handle transform animations
        for (_e, anim, transform) in engine.world.query2_mut::<Animation, Transform>() {
            if !anim.playing || anim.clip.is_empty() {
                continue;
            }
            if let Some(clip) = self.assets.get_animation(&anim.clip) {
                anim.time += delta;
                if clip.duration > 0.0 {
                    anim.time = anim.time % clip.duration;
                }

                // Apply each animation channel
                for channel in &clip.channels {
                    match channel {
                        AnimationChannel::Translation(keyframes) => {
                            if let Some(value) = Self::interpolate_vec3(keyframes, anim.time) {
                                // Apply both the object's scale and the animation's translation scale
                                let object_scale = (transform.size[0] + transform.size[1] + transform.size[2]) / 3.0;
                                let total_scale = object_scale * anim.translation_scale;
                                transform.position = [
                                    value[0] * total_scale,
                                    value[1] * total_scale,
                                    value[2] * total_scale,
                                ];
                            }
                        }
                        AnimationChannel::Rotation(keyframes) => {
                            if let Some(value) = Self::interpolate_quat(keyframes, anim.time) {
                                transform.orientation = value;
                            }
                        }
                        AnimationChannel::Scale(keyframes) => {
                            if let Some(value) = Self::interpolate_vec3(keyframes, anim.time) {
                                transform.size = value;
                            }
                        }
                        AnimationChannel::MorphTargetWeights(_) => {
                            // Morph weights are handled separately below
                        }
                    }
                }
            }
        }

        // Handle morph target weight animations
        for (_e, anim, morph_weights) in engine.world.query2_mut::<Animation, MorphWeights>() {
            if !anim.playing || anim.clip.is_empty() {
                continue;
            }
            if let Some(clip) = self.assets.get_animation(&anim.clip) {
                // Find morph target weight channels
                for channel in &clip.channels {
                    if let AnimationChannel::MorphTargetWeights(keyframes) = channel {
                        if let Some(weights) = Self::interpolate_morph_weights(keyframes, anim.time) {
                            morph_weights.weights = weights;
                        }
                    }
                }
            }
        }
    }
}

impl AnimationSystem {
    /// Interpolate between Vec3 keyframes at the given time
    fn interpolate_vec3(keyframes: &[(f32, [f32; 3])], time: f32) -> Option<[f32; 3]> {
        if keyframes.is_empty() {
            return None;
        }

        if keyframes.len() == 1 {
            return Some(keyframes[0].1);
        }

        // Find the keyframes to interpolate between
        let mut prev = keyframes[0];
        let mut next = keyframes[keyframes.len() - 1];

        for kf in keyframes.iter() {
            if kf.0 <= time {
                prev = *kf;
            }
            if kf.0 >= time {
                next = *kf;
                break;
            }
        }

        // Linear interpolation
        let span = (next.0 - prev.0).max(f32::EPSILON);
        let t = (time - prev.0) / span;

        Some([
            prev.1[0] + (next.1[0] - prev.1[0]) * t,
            prev.1[1] + (next.1[1] - prev.1[1]) * t,
            prev.1[2] + (next.1[2] - prev.1[2]) * t,
        ])
    }

    /// Interpolate between quaternion keyframes at the given time using SLERP
    fn interpolate_quat(keyframes: &[(f32, [f32; 4])], time: f32) -> Option<[f32; 4]> {
        if keyframes.is_empty() {
            return None;
        }

        if keyframes.len() == 1 {
            return Some(keyframes[0].1);
        }

        // Find the keyframes to interpolate between
        let mut prev = keyframes[0];
        let mut next = keyframes[keyframes.len() - 1];

        for kf in keyframes.iter() {
            if kf.0 <= time {
                prev = *kf;
            }
            if kf.0 >= time {
                next = *kf;
                break;
            }
        }

        // Spherical linear interpolation (SLERP) for quaternions
        let span = (next.0 - prev.0).max(f32::EPSILON);
        let t = (time - prev.0) / span;

        Some(Self::slerp_quat(prev.1, next.1, t))
    }

    /// Spherical linear interpolation between two quaternions
    fn slerp_quat(q1: [f32; 4], q2: [f32; 4], t: f32) -> [f32; 4] {
        // Compute dot product
        let mut dot = q1[0] * q2[0] + q1[1] * q2[1] + q1[2] * q2[2] + q1[3] * q2[3];

        // If dot product is negative, negate one quaternion to take shorter path
        let q2 = if dot < 0.0 {
            dot = -dot;
            [-q2[0], -q2[1], -q2[2], -q2[3]]
        } else {
            q2
        };

        // If quaternions are very close, use linear interpolation
        if dot > 0.9995 {
            let result = [
                q1[0] + t * (q2[0] - q1[0]),
                q1[1] + t * (q2[1] - q1[1]),
                q1[2] + t * (q2[2] - q1[2]),
                q1[3] + t * (q2[3] - q1[3]),
            ];
            // Normalize
            let len = (result[0] * result[0] + result[1] * result[1] + result[2] * result[2] + result[3] * result[3]).sqrt();
            if len > 0.0 {
                [result[0] / len, result[1] / len, result[2] / len, result[3] / len]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            }
        } else {
            // Use SLERP
            let theta = dot.acos();
            let sin_theta = theta.sin();
            let w1 = ((1.0 - t) * theta).sin() / sin_theta;
            let w2 = (t * theta).sin() / sin_theta;

            [
                w1 * q1[0] + w2 * q2[0],
                w1 * q1[1] + w2 * q2[1],
                w1 * q1[2] + w2 * q2[2],
                w1 * q1[3] + w2 * q2[3],
            ]
        }
    }

    /// Interpolate between morph target weight keyframes at the given time
    fn interpolate_morph_weights(keyframes: &[(f32, Vec<f32>)], time: f32) -> Option<Vec<f32>> {
        if keyframes.is_empty() {
            return None;
        }

        if keyframes.len() == 1 {
            return Some(keyframes[0].1.clone());
        }

        // Find the keyframes to interpolate between
        let mut prev = &keyframes[0];
        let mut next = &keyframes[keyframes.len() - 1];

        for kf in keyframes.iter() {
            if kf.0 <= time {
                prev = kf;
            }
            if kf.0 >= time {
                next = kf;
                break;
            }
        }

        // Linear interpolation between weight arrays
        let span = (next.0 - prev.0).max(f32::EPSILON);
        let t = (time - prev.0) / span;

        // Ensure both weight arrays have the same length
        let max_len = prev.1.len().max(next.1.len());
        let mut result = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let prev_weight = prev.1.get(i).copied().unwrap_or(0.0);
            let next_weight = next.1.get(i).copied().unwrap_or(0.0);
            result.push(prev_weight + (next_weight - prev_weight) * t);
        }

        Some(result)
    }
}
