use super::*;

pub(super) fn reflection_grid_cell(value: Vec3, cell_size: f32) -> (i32, i32, i32) {
    let inv = 1.0 / cell_size.max(0.25);
    (
        (value.x * inv).floor() as i32,
        (value.y * inv).floor() as i32,
        (value.z * inv).floor() as i32,
    )
}

pub(super) fn reflection_probe_world_bounds(probe: &RenderReflectionProbe) -> (Vec3, Vec3) {
    let extents = probe.half_extents.max(Vec3::splat(0.001));
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    for x in [-extents.x, extents.x] {
        for y in [-extents.y, extents.y] {
            for z in [-extents.z, extents.z] {
                let world = probe.probe_to_world.transform_point3(Vec3::new(x, y, z));
                minimum = minimum.min(world);
                maximum = maximum.max(world);
            }
        }
    }
    (minimum, maximum)
}

impl WgpuRenderer {
    pub(super) fn rebuild_reflection_probe_spatial_index(&mut self, frame: &RenderFrame) {
        let cell_size = frame.settings.reflection_probe_grid_cell_size.max(0.25);
        self.environment.reflection_probe_spatial_index.cell_size = cell_size;
        self.environment.reflection_probe_spatial_index.cells.clear();
        self.environment.reflection_probe_spatial_index.oversized.clear();
        for (index, probe) in frame.reflection_probes.iter().take(MAX_REFLECTION_PROBES).enumerate() {
            let (minimum, maximum) = reflection_probe_world_bounds(probe);
            let first = reflection_grid_cell(minimum, cell_size);
            let last = reflection_grid_cell(maximum, cell_size);
            let cell_count = (last.0 as i64 - first.0 as i64 + 1)
                .saturating_mul(last.1 as i64 - first.1 as i64 + 1)
                .saturating_mul(last.2 as i64 - first.2 as i64 + 1);
            const MAX_CELLS_PER_PROBE: i64 = 4_096;
            if cell_count > MAX_CELLS_PER_PROBE {
                self.environment.reflection_probe_spatial_index.oversized.push(index as u32);
                continue;
            }
            for z in first.2..=last.2 {
                for y in first.1..=last.1 {
                    for x in first.0..=last.0 {
                        self.environment.reflection_probe_spatial_index
                            .cells
                            .entry((x, y, z))
                            .or_default()
                            .push(index as u32);
                    }
                }
            }
        }
    }

    pub(super) fn reflection_probe_spatial_candidates(&self, bounds_min: Vec3, bounds_max: Vec3) -> Vec<u32> {
        let cell_size = self.environment.reflection_probe_spatial_index.cell_size.max(0.25);
        let first = reflection_grid_cell(bounds_min, cell_size);
        let last = reflection_grid_cell(bounds_max, cell_size);
        let query_cell_count = (last.0 as i64 - first.0 as i64 + 1)
            .saturating_mul(last.1 as i64 - first.1 as i64 + 1)
            .saturating_mul(last.2 as i64 - first.2 as i64 + 1);
        const MAX_QUERY_CELLS: i64 = 4_096;
        if query_cell_count > MAX_QUERY_CELLS {
            return (0..MAX_REFLECTION_PROBES as u32).collect();
        }
        let mut seen = [false; MAX_REFLECTION_PROBES];
        let mut candidates = Vec::new();
        for &index in &self.environment.reflection_probe_spatial_index.oversized {
            let index_usize = index as usize;
            if index_usize < MAX_REFLECTION_PROBES && !seen[index_usize] {
                seen[index_usize] = true;
                candidates.push(index);
            }
        }
        for z in first.2..=last.2 {
            for y in first.1..=last.1 {
                for x in first.0..=last.0 {
                    let Some(indices) = self.environment.reflection_probe_spatial_index.cells.get(&(x, y, z)) else {
                        continue;
                    };
                    for &index in indices {
                        let index_usize = index as usize;
                        if index_usize < MAX_REFLECTION_PROBES && !seen[index_usize] {
                            seen[index_usize] = true;
                            candidates.push(index);
                        }
                    }
                }
            }
        }
        candidates
    }
}
