//! Clean runtime systems implemented against the new `vetrace_core` ECS.

use crate::components::{
    Animation, AnimationInterpolation, AnimationOutputValues, AnimationPlayer, AnimationSampler,
    AnimationTargetProperty, Easing, Lerp, LerpState, LoopMode, MorphWeights,
};
use glam::{Quat, Vec3};
use vetrace_core::{propagate_global_transforms, Engine, Transform};
use vetrace_core::ecs::Entity;

fn ease(t: f32, easing: Easing) -> f32 {
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
        }
    }
}

pub fn update_animation_clocks(engine: &mut Engine, dt: f32) {
    for (_, animation) in engine.raw_world_mut().query_mut::<Animation>() {
        if !animation.playing { continue; }
        animation.time_seconds += dt * animation.speed;
        if animation.length_seconds <= 0.0 { continue; }
        if animation.time_seconds >= animation.length_seconds {
            match animation.loop_mode {
                LoopMode::Once => {
                    animation.time_seconds = animation.length_seconds;
                    animation.playing = false;
                }
                LoopMode::Repeat => animation.time_seconds %= animation.length_seconds,
                LoopMode::PingPong => {
                    animation.time_seconds = animation.length_seconds;
                    animation.speed = -animation.speed.abs();
                }
            }
        } else if animation.time_seconds <= 0.0 && animation.speed < 0.0 {
            match animation.loop_mode {
                LoopMode::Once => {
                    animation.time_seconds = 0.0;
                    animation.playing = false;
                }
                LoopMode::Repeat => animation.time_seconds = animation.length_seconds,
                LoopMode::PingPong => {
                    animation.time_seconds = 0.0;
                    animation.speed = animation.speed.abs();
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum SampledValue {
    Translation(Entity, Vec3),
    Rotation(Entity, Quat),
    Scale(Entity, Vec3),
    MorphWeights(Entity, Vec<f32>),
}

/// Advances all `AnimationPlayer` components and writes sampled values back into
/// ordinary core `Transform` / animation `MorphWeights` components.
pub fn update_animation_players(engine: &mut Engine, dt: f32) {
    let mut sampled = Vec::new();
    let safe_dt = dt.max(0.0);

    for (_, player) in engine.raw_world_mut().query_mut::<AnimationPlayer>() {
        if player.clips.is_empty() || player.active_clip >= player.clips.len() {
            continue;
        }

        let duration = player.clips[player.active_clip].duration.max(0.0);
        if player.playing {
            advance_player_time(player, safe_dt, duration);
        }

        let time = player.time_seconds;
        let Some(clip) = player.clips.get(player.active_clip) else { continue; };
        for channel in &clip.channels {
            match (channel.property, sample_channel(&channel.sampler, time)) {
                (AnimationTargetProperty::Translation, Some(SampledChannelValue::Vec3(value))) => {
                    sampled.push(SampledValue::Translation(channel.target, value));
                }
                (AnimationTargetProperty::Rotation, Some(SampledChannelValue::Quat(value))) => {
                    sampled.push(SampledValue::Rotation(channel.target, value.normalize()));
                }
                (AnimationTargetProperty::Scale, Some(SampledChannelValue::Vec3(value))) => {
                    sampled.push(SampledValue::Scale(channel.target, value));
                }
                (AnimationTargetProperty::MorphWeights, Some(SampledChannelValue::Weights(values))) => {
                    sampled.push(SampledValue::MorphWeights(channel.target, values));
                }
                _ => {}
            }
        }
    }

    if sampled.is_empty() {
        return;
    }

    let mut touched_transforms = false;
    for value in sampled {
        match value {
            SampledValue::Translation(entity, translation) => {
                if let Some(transform) = ensure_transform(engine, entity) {
                    transform.translation = translation;
                    touched_transforms = true;
                }
            }
            SampledValue::Rotation(entity, rotation) => {
                if let Some(transform) = ensure_transform(engine, entity) {
                    transform.rotation = rotation.normalize();
                    touched_transforms = true;
                }
            }
            SampledValue::Scale(entity, scale) => {
                if let Some(transform) = ensure_transform(engine, entity) {
                    transform.scale = scale;
                    touched_transforms = true;
                }
            }
            SampledValue::MorphWeights(entity, weights) => {
                if let Some(current) = engine.raw_world_mut().get_mut::<MorphWeights>(entity) {
                    current.weights = weights;
                } else if engine.raw_world().is_alive(entity) {
                    engine.raw_world_mut().insert(entity, MorphWeights { weights });
                }
            }
        }
    }

    if touched_transforms {
        propagate_global_transforms(engine);
    }
}

fn ensure_transform(engine: &mut Engine, entity: Entity) -> Option<&mut Transform> {
    if !engine.raw_world().is_alive(entity) {
        return None;
    }
    if engine.raw_world().get::<Transform>(entity).is_none() {
        engine.raw_world_mut().insert(entity, Transform::default());
    }
    engine.raw_world_mut().get_mut::<Transform>(entity)
}

fn advance_player_time(player: &mut AnimationPlayer, dt: f32, duration: f32) {
    player.time_seconds += dt * player.speed;
    if duration <= 0.0 {
        player.time_seconds = 0.0;
        return;
    }

    if player.time_seconds >= duration {
        match player.loop_mode {
            LoopMode::Once => {
                player.time_seconds = duration;
                player.playing = false;
            }
            LoopMode::Repeat => player.time_seconds %= duration,
            LoopMode::PingPong => {
                player.time_seconds = duration;
                player.speed = -player.speed.abs();
            }
        }
    } else if player.time_seconds <= 0.0 && player.speed < 0.0 {
        match player.loop_mode {
            LoopMode::Once => {
                player.time_seconds = 0.0;
                player.playing = false;
            }
            LoopMode::Repeat => player.time_seconds = duration,
            LoopMode::PingPong => {
                player.time_seconds = 0.0;
                player.speed = player.speed.abs();
            }
        }
    }
}

#[derive(Clone, Debug)]
enum SampledChannelValue {
    Vec3(Vec3),
    Quat(Quat),
    Weights(Vec<f32>),
}

fn sample_channel(sampler: &AnimationSampler, time: f32) -> Option<SampledChannelValue> {
    if sampler.inputs.is_empty() || sampler.outputs.key_count() == 0 {
        return None;
    }
    let key_count = sampler.inputs.len().min(sampler.outputs.key_count());
    if key_count == 0 {
        return None;
    }
    if key_count == 1 || time <= sampler.inputs[0] {
        return sample_key(&sampler.outputs, 0);
    }
    if time >= sampler.inputs[key_count - 1] {
        return sample_key(&sampler.outputs, key_count - 1);
    }

    let hi = sampler.inputs[..key_count]
        .partition_point(|sample_time| *sample_time <= time)
        .min(key_count - 1);
    let lo = hi.saturating_sub(1);
    let t0 = sampler.inputs[lo];
    let t1 = sampler.inputs[hi];
    let alpha = if t1 > t0 { ((time - t0) / (t1 - t0)).clamp(0.0, 1.0) } else { 0.0 };

    match sampler.interpolation {
        AnimationInterpolation::Step => sample_key(&sampler.outputs, lo),
        AnimationInterpolation::Linear | AnimationInterpolation::CubicSpline => interpolate_keys(&sampler.outputs, lo, hi, alpha),
    }
}

fn sample_key(outputs: &AnimationOutputValues, index: usize) -> Option<SampledChannelValue> {
    match outputs {
        AnimationOutputValues::Vec3(values) => values.get(index).copied().map(SampledChannelValue::Vec3),
        AnimationOutputValues::Quat(values) => values.get(index).copied().map(SampledChannelValue::Quat),
        AnimationOutputValues::Weights { width, values } => {
            if *width == 0 { return None; }
            let start = index.checked_mul(*width)?;
            let end = start.checked_add(*width)?;
            values.get(start..end).map(|slice| SampledChannelValue::Weights(slice.to_vec()))
        }
    }
}

fn interpolate_keys(outputs: &AnimationOutputValues, lo: usize, hi: usize, alpha: f32) -> Option<SampledChannelValue> {
    match outputs {
        AnimationOutputValues::Vec3(values) => Some(SampledChannelValue::Vec3(values.get(lo)?.lerp(*values.get(hi)?, alpha))),
        AnimationOutputValues::Quat(values) => Some(SampledChannelValue::Quat(values.get(lo)?.slerp(*values.get(hi)?, alpha).normalize())),
        AnimationOutputValues::Weights { width, values } => {
            if *width == 0 { return None; }
            let lo_start = lo.checked_mul(*width)?;
            let hi_start = hi.checked_mul(*width)?;
            let lo_slice = values.get(lo_start..lo_start + *width)?;
            let hi_slice = values.get(hi_start..hi_start + *width)?;
            let mut out = Vec::with_capacity(*width);
            for i in 0..*width {
                out.push(lo_slice[i] + (hi_slice[i] - lo_slice[i]) * alpha);
            }
            Some(SampledChannelValue::Weights(out))
        }
    }
}

pub fn update_lerps(engine: &mut Engine, dt: f32) {
    for (_, lerp) in engine.raw_world_mut().query_mut::<Lerp>() {
        for channel in &mut lerp.channels {
            if channel.state != LerpState::Playing { continue; }
            channel.elapsed = (channel.elapsed + dt).max(0.0);
            let duration = channel.duration.max(f32::EPSILON);
            if channel.elapsed >= duration {
                match channel.loop_mode {
                    LoopMode::Once => {
                        channel.elapsed = duration;
                        channel.state = LerpState::Finished;
                    }
                    LoopMode::Repeat => channel.elapsed %= duration,
                    LoopMode::PingPong => {
                        std::mem::swap(&mut channel.from, &mut channel.to);
                        channel.elapsed = 0.0;
                    }
                }
            }
            let _value = channel.from + (channel.to - channel.from) * ease((channel.elapsed / duration).clamp(0.0, 1.0), channel.easing);
            // The channel value is intentionally not written to a hard-coded target.
            // User/runtime plugins decide how to map lerp channels onto transforms,
            // UI properties, materials, audio volume, etc.
        }
    }
}
