use crate::scene::object::GpuTriangle;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct GpuTriBvhNode {
    pub bounds_min: [f32; 4],
    pub bounds_max: [f32; 4],
    pub children: [i32; 4],
}

/// Offset child node indices of a BVH by the given start index. This is
/// required when multiple meshes share a single GPU buffer so that all child
/// pointers reference the correct global indices.
pub fn offset_nodes(nodes: &mut [GpuTriBvhNode], start: i32) {
    for n in nodes {
        for c in &mut n.children[0..2] {
            if *c >= 0 {
                *c += start;
            }
        }
    }
}

/// Compute axis aligned bounding box for a set of triangles.
pub fn mesh_bounds(tris: &[GpuTriangle]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for t in tris {
        let v1 = [t.v0[0] + t.e1[0], t.v0[1] + t.e1[1], t.v0[2] + t.e1[2]];
        let v2 = [t.v0[0] + t.e2[0], t.v0[1] + t.e2[1], t.v0[2] + t.e2[2]];
        for d in 0..3 {
            let vmin = t.v0[d].min(v1[d]).min(v2[d]);
            let vmax = t.v0[d].max(v1[d]).max(v2[d]);
            if vmin < min[d] {
                min[d] = vmin;
            }
            if vmax > max[d] {
                max[d] = vmax;
            }
        }
    }
    (min, max)
}
pub fn build_bvh(tris: &[GpuTriangle]) -> Vec<GpuTriBvhNode> {
    #[derive(Clone)]
    struct TriEntry {
        index: usize,
        min: [f32; 3],
        max: [f32; 3],
    }

    fn tri_bounds(t: &GpuTriangle) -> ([f32; 3], [f32; 3]) {
        let v1 = [t.v0[0] + t.e1[0], t.v0[1] + t.e1[1], t.v0[2] + t.e1[2]];
        let v2 = [t.v0[0] + t.e2[0], t.v0[1] + t.e2[1], t.v0[2] + t.e2[2]];
        let min = [
            t.v0[0].min(v1[0]).min(v2[0]),
            t.v0[1].min(v1[1]).min(v2[1]),
            t.v0[2].min(v1[2]).min(v2[2]),
        ];
        let max = [
            t.v0[0].max(v1[0]).max(v2[0]),
            t.v0[1].max(v1[1]).max(v2[1]),
            t.v0[2].max(v1[2]).max(v2[2]),
        ];
        (min, max)
    }

    fn bounds(entries: &[TriEntry]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for e in entries {
            for d in 0..3 {
                if e.min[d] < min[d] {
                    min[d] = e.min[d];
                }
                if e.max[d] > max[d] {
                    max[d] = e.max[d];
                }
            }
        }
        (min, max)
    }

    fn build(nodes: &mut Vec<GpuTriBvhNode>, entries: &[TriEntry]) -> i32 {
        if entries.is_empty() {
            return -1;
        }
        if entries.len() == 1 {
            let idx = nodes.len() as i32;
            nodes.push(GpuTriBvhNode {
                bounds_min: [entries[0].min[0], entries[0].min[1], entries[0].min[2], 0.0],
                bounds_max: [entries[0].max[0], entries[0].max[1], entries[0].max[2], 0.0],
                children: [-1, -1, entries[0].index as i32, 0],
            });
            return idx;
        }
        let (min, max) = bounds(entries);
        let idx = nodes.len() as i32;
        nodes.push(GpuTriBvhNode {
            bounds_min: [min[0], min[1], min[2], 0.0],
            bounds_max: [max[0], max[1], max[2], 0.0],
            children: [-1, -1, -1, 0],
        });
        let axis = {
            let ext = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
            if ext[0] >= ext[1] && ext[0] >= ext[2] {
                0
            } else if ext[1] >= ext[0] && ext[1] >= ext[2] {
                1
            } else {
                2
            }
        };
        let mut inds = entries.to_vec();
        inds.sort_by(|a, b| {
            ((a.min[axis] + a.max[axis]) * 0.5)
                .partial_cmp(&((b.min[axis] + b.max[axis]) * 0.5))
                .unwrap()
        });
        let mid = inds.len() / 2;
        let left = build(nodes, &inds[..mid]);
        let right = build(nodes, &inds[mid..]);
        nodes[idx as usize].children = [left, right, -1, 0];
        idx
    }

    let entries: Vec<TriEntry> = tris
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let (min, max) = tri_bounds(t);
            TriEntry { index: i, min, max }
        })
        .collect();
    let mut nodes = Vec::new();
    build(&mut nodes, &entries);
    nodes
}