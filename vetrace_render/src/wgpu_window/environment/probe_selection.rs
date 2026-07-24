use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct ReflectionProbeCandidate {
    pub(super) entity: u64,
    pub(super) index: u32,
    pub(super) priority: i32,
    pub(super) score: f32,
}

pub(super) fn reflection_probe_candidate_cmp(
    a: &ReflectionProbeCandidate,
    b: &ReflectionProbeCandidate,
) -> std::cmp::Ordering {
    b.priority
        .cmp(&a.priority)
        .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
        .then_with(|| a.entity.cmp(&b.entity))
}

impl WgpuRenderer {
    pub(super) fn selected_reflection_probe_indices(
        &mut self,
        frame: &RenderFrame,
        pending: &PendingDraw<'_>,
        scene_frame: u64,
    ) -> ([u32; 4], usize) {
        let mut candidates = Vec::new();
        let spatial_candidates = self.reflection_probe_spatial_candidates(pending.bounds_min, pending.bounds_max);
        for index in spatial_candidates {
            let Some(probe) = frame.reflection_probes.get(index as usize) else { continue; };
            let layers = pending.object.render_layers;
            if layers & probe.include_layers == 0 || layers & probe.exclude_layers != 0 {
                continue;
            }
            let Some(score) = probe_overlap_score(probe, pending.bounds_min, pending.bounds_max) else {
                continue;
            };
            candidates.push(ReflectionProbeCandidate {
                entity: probe.entity.0,
                index,
                priority: probe.priority,
                score,
            });
        }
        candidates.sort_by(reflection_probe_candidate_cmp);

        let cache_key = pending.object.entity.0;
        let previous = self
            .environment
            .reflection_probe_selection_cache
            .get(&cache_key)
            .copied()
            .unwrap_or_default();

        // Preserve still-valid candidates until a replacement is materially
        // better. This prevents the fourth probe from thrashing at volume seams.
        let mut selected = Vec::with_capacity(4);
        for entity in previous.entities.into_iter().take(previous.count) {
            if let Some(candidate) = candidates.iter().find(|candidate| candidate.entity == entity) {
                selected.push(*candidate);
            }
        }

        for desired in candidates.iter().take(4).copied() {
            if selected.iter().any(|candidate| candidate.entity == desired.entity) {
                continue;
            }
            if selected.len() < 4 {
                selected.push(desired);
                continue;
            }

            let weakest_index = selected
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| reflection_probe_candidate_cmp(a, b))
                .map(|(index, _)| index)
                .unwrap_or(0);
            let weakest = selected[weakest_index];
            let replace = desired.priority > weakest.priority
                || (desired.priority == weakest.priority
                    && desired.score > weakest.score * 1.12 + 0.02);
            if replace {
                selected[weakest_index] = desired;
            }
        }

        selected.sort_by(reflection_probe_candidate_cmp);
        selected.truncate(4);

        let mut indices = [u32::MAX; 4];
        let mut entities = [u64::MAX; 4];
        for (lane, candidate) in selected.iter().enumerate() {
            indices[lane] = candidate.index;
            entities[lane] = candidate.entity;
        }
        let count = selected.len();
        self.environment.reflection_probe_selection_cache.insert(
            cache_key,
            CachedReflectionProbeSelection {
                entities,
                count,
                last_used_frame: scene_frame,
            },
        );
        indices
            .iter()
            .take(count)
            .for_each(|index| debug_assert!(*index < MAX_REFLECTION_PROBES as u32));
        (indices, count)
    }

    pub(super) fn prune_reflection_probe_selection_cache(&mut self, scene_frame: u64) {
        const CACHE_TTL_FRAMES: u64 = 120;
        self.environment.reflection_probe_selection_cache.retain(|_, cached| {
            scene_frame.saturating_sub(cached.last_used_frame) <= CACHE_TTL_FRAMES
        });
    }
}

pub(super) fn probe_overlap_score(
    probe: &crate::backend::RenderReflectionProbe,
    bounds_min: Vec3,
    bounds_max: Vec3,
) -> Option<f32> {
    let corners = [
        Vec3::new(bounds_min.x, bounds_min.y, bounds_min.z),
        Vec3::new(bounds_max.x, bounds_min.y, bounds_min.z),
        Vec3::new(bounds_min.x, bounds_max.y, bounds_min.z),
        Vec3::new(bounds_max.x, bounds_max.y, bounds_min.z),
        Vec3::new(bounds_min.x, bounds_min.y, bounds_max.z),
        Vec3::new(bounds_max.x, bounds_min.y, bounds_max.z),
        Vec3::new(bounds_min.x, bounds_max.y, bounds_max.z),
        Vec3::new(bounds_max.x, bounds_max.y, bounds_max.z),
    ];
    let mut local_min = Vec3::splat(f32::INFINITY);
    let mut local_max = Vec3::splat(f32::NEG_INFINITY);
    for corner in corners {
        let local = probe.world_to_probe.transform_point3(corner);
        local_min = local_min.min(local);
        local_max = local_max.max(local);
    }
    if local_max.x < -probe.half_extents.x
        || local_min.x > probe.half_extents.x
        || local_max.y < -probe.half_extents.y
        || local_min.y > probe.half_extents.y
        || local_max.z < -probe.half_extents.z
        || local_min.z > probe.half_extents.z
    {
        return None;
    }

    let center = (bounds_min + bounds_max) * 0.5;
    let local_center = probe.world_to_probe.transform_point3(center).abs();
    let normalized = local_center / probe.half_extents.max(Vec3::splat(0.001));
    let edge = normalized.max_element();
    Some((1.0 - edge).clamp(0.0, 1.0))
}
