use std::collections::HashMap;

use super::*;
use vetrace_animation::{
    AnimationChannel, AnimationClip, AnimationInterpolation, AnimationOutputValues, AnimationPlayer,
    AnimationSampler, AnimationTargetProperty, MorphWeights, Skin,
};

pub(crate) fn import_skins(
    engine: &mut Engine,
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    node_entities: &HashMap<usize, Entity>,
) {
    for node in document.nodes() {
        let Some(skin) = node.skin() else { continue; };
        let Some(&entity) = node_entities.get(&node.index()) else { continue; };
        let joints: Vec<Entity> = skin
            .joints()
            .filter_map(|joint| node_entities.get(&joint.index()).copied())
            .collect();
        if joints.is_empty() {
            continue;
        }
        let inverse_bind_matrices: Vec<glam::Mat4> = skin
            .reader(|buffer| buffers.get(buffer.index()).map(|data| &**data))
            .read_inverse_bind_matrices()
            .map(|matrices| matrices.map(|m| glam::Mat4::from_cols_array_2d(&m)).collect())
            .unwrap_or_else(|| vec![glam::Mat4::IDENTITY; joints.len()]);
        let skeleton_root = skin.skeleton().and_then(|skeleton| node_entities.get(&skeleton.index()).copied());
        engine.raw_world_mut().insert(entity, Skin { joints, inverse_bind_matrices, skeleton_root });
    }
}

pub(crate) fn import_animations(
    engine: &mut Engine,
    document: &gltf::Document,
    buffers: &[gltf::buffer::Data],
    root: Entity,
    node_entities: &HashMap<usize, Entity>,
    report: &mut GltfLoadReport,
) -> Result<()> {
    let mut clips = Vec::new();

    for animation in document.animations() {
        let mut clip = AnimationClip {
            name: animation
                .name()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("gltf_animation_{}", animation.index())),
            duration: 0.0,
            channels: Vec::new(),
        };

        for channel in animation.channels() {
            let target_node = channel.target().node();
            let Some(&target_entity) = node_entities.get(&target_node.index()) else {
                continue;
            };
            let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(|data| &**data));
            let Some(inputs_iter) = reader.read_inputs() else {
                continue;
            };
            let inputs: Vec<f32> = inputs_iter.collect();
            if inputs.is_empty() {
                continue;
            }

            let interpolation = interpolation_from_gltf(channel.sampler().interpolation());
            let Some(read_outputs) = reader.read_outputs() else {
                continue;
            };
            let maybe_outputs = match read_outputs {
                gltf::animation::util::ReadOutputs::Translations(values) => {
                    let values: Vec<Vec3> = values.map(Vec3::from_array).collect();
                    let values = strip_cubic_vec3(values, interpolation, inputs.len());
                    Some((AnimationTargetProperty::Translation, AnimationOutputValues::Vec3(values)))
                }
                gltf::animation::util::ReadOutputs::Rotations(values) => {
                    let values: Vec<Quat> = values
                        .into_f32()
                        .map(|q| Quat::from_xyzw(q[0], q[1], q[2], q[3]).normalize())
                        .collect();
                    let values = strip_cubic_quat(values, interpolation, inputs.len());
                    Some((AnimationTargetProperty::Rotation, AnimationOutputValues::Quat(values)))
                }
                gltf::animation::util::ReadOutputs::Scales(values) => {
                    let values: Vec<Vec3> = values.map(Vec3::from_array).collect();
                    let values = strip_cubic_vec3(values, interpolation, inputs.len());
                    Some((AnimationTargetProperty::Scale, AnimationOutputValues::Vec3(values)))
                }
                gltf::animation::util::ReadOutputs::MorphTargetWeights(values) => {
                    let raw: Vec<f32> = values.into_f32().collect();
                    let target_width = channel
                        .target()
                        .node()
                        .mesh()
                        .and_then(|mesh| mesh.weights().map(|weights| weights.len()))
                        .unwrap_or(0);
                    let (width, values) = strip_cubic_weights(raw, interpolation, inputs.len(), target_width);
                    (width > 0).then_some((AnimationTargetProperty::MorphWeights, AnimationOutputValues::Weights { width, values }))
                }
            };
            let Some((property, outputs)) = maybe_outputs else {
                continue;
            };
            let sampler = AnimationSampler { inputs, interpolation, outputs };
            clip.duration = clip.duration.max(sampler.duration());
            clip.channels.push(AnimationChannel { target: target_entity, property, sampler });
        }

        if !clip.channels.is_empty() {
            report.animations_loaded += 1;
            report.animation_channels_loaded += clip.channels.len();
            clips.push(clip);
        }
    }

    if !clips.is_empty() {
        engine.raw_world_mut().insert(root, AnimationPlayer::with_clips(clips));
    }

    Ok(())
}

fn interpolation_from_gltf(value: gltf::animation::Interpolation) -> AnimationInterpolation {
    match value {
        gltf::animation::Interpolation::Step => AnimationInterpolation::Step,
        gltf::animation::Interpolation::Linear => AnimationInterpolation::Linear,
        gltf::animation::Interpolation::CubicSpline => AnimationInterpolation::CubicSpline,
    }
}

fn strip_cubic_vec3(values: Vec<Vec3>, interpolation: AnimationInterpolation, input_count: usize) -> Vec<Vec3> {
    if interpolation == AnimationInterpolation::CubicSpline && values.len() == input_count.saturating_mul(3) {
        values.chunks_exact(3).map(|chunk| chunk[1]).collect()
    } else {
        values
    }
}

fn strip_cubic_quat(values: Vec<Quat>, interpolation: AnimationInterpolation, input_count: usize) -> Vec<Quat> {
    if interpolation == AnimationInterpolation::CubicSpline && values.len() == input_count.saturating_mul(3) {
        values.chunks_exact(3).map(|chunk| chunk[1].normalize()).collect()
    } else {
        values
    }
}

fn strip_cubic_weights(
    raw: Vec<f32>,
    interpolation: AnimationInterpolation,
    input_count: usize,
    target_width: usize,
) -> (usize, Vec<f32>) {
    if input_count == 0 || raw.is_empty() {
        return (0, Vec::new());
    }

    if interpolation == AnimationInterpolation::CubicSpline {
        let width = if target_width > 0 {
            target_width
        } else {
            raw.len().checked_div(input_count.saturating_mul(3)).unwrap_or(0)
        };
        if width == 0 || raw.len() < input_count.saturating_mul(width).saturating_mul(3) {
            return (0, Vec::new());
        }
        let mut values = Vec::with_capacity(input_count * width);
        let per_key = width * 3;
        for key in 0..input_count {
            let base = key * per_key + width;
            values.extend_from_slice(raw.get(base..base + width).unwrap_or(&[]));
        }
        return (width, values);
    }

    let width = if target_width > 0 { target_width } else { raw.len().checked_div(input_count).unwrap_or(0) };
    if width == 0 {
        return (0, Vec::new());
    }
    let wanted = input_count.saturating_mul(width).min(raw.len());
    (width, raw.get(..wanted).unwrap_or(&[]).to_vec())
}

pub(crate) fn attach_initial_morph_weights(engine: &mut Engine, entity: Entity, node: &gltf::Node<'_>) {
    if engine.raw_world().get::<MorphWeights>(entity).is_some() {
        return;
    }
    let Some(mesh) = node.mesh() else { return; };
    let Some(weights) = mesh.weights() else { return; };
    let weights: Vec<f32> = weights.to_vec();
    if !weights.is_empty() {
        engine.raw_world_mut().insert(entity, MorphWeights { weights });
    }
}
