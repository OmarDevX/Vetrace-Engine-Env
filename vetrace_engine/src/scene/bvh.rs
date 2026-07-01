use crate::scene::object::{GpuTriangle, Object};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct GpuBvhNode {
    pub bounds_min: [f32; 4],
    pub bounds_max: [f32; 4],
    pub children: [i32; 4],
}

#[derive(Clone, Copy)]
struct ObjBounds {
    index: usize,
    min: [f32; 3],
    max: [f32; 3],
    center: [f32; 3],
}

fn rotate_vec(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let qv = [q[0], q[1], q[2]];
    let uv = [
        qv[1] * v[2] - qv[2] * v[1],
        qv[2] * v[0] - qv[0] * v[2],
        qv[0] * v[1] - qv[1] * v[0],
    ];
    let uuv = [
        qv[1] * uv[2] - qv[2] * uv[1],
        qv[2] * uv[0] - qv[0] * uv[2],
        qv[0] * uv[1] - qv[1] * uv[0],
    ];
    [
        v[0] + 2.0 * (q[3] * uv[0] + uuv[0]),
        v[1] + 2.0 * (q[3] * uv[1] + uuv[1]),
        v[2] + 2.0 * (q[3] * uv[2] + uuv[2]),
    ]
}

fn include_point(min: &mut [f32; 3], max: &mut [f32; 3], p: [f32; 3]) {
    for d in 0..3 {
        min[d] = min[d].min(p[d]);
        max[d] = max[d].max(p[d]);
    }
}

fn transform_point(o: &Object, p: [f32; 3]) -> [f32; 3] {
    let scaled = [p[0] * o.scale[0], p[1] * o.scale[1], p[2] * o.scale[2]];
    let rotated = rotate_vec(o.orientation, scaled);
    [
        rotated[0] + o.position[0],
        rotated[1] + o.position[1],
        rotated[2] + o.position[2],
    ]
}

fn object_bounds(o: &Object, triangles: &[GpuTriangle], index: usize) -> ObjBounds {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    if o.is_mesh && o.triangle_count > 0 {
        let end = (o.triangle_start_idx + o.triangle_count).min(triangles.len());
        for t in &triangles[o.triangle_start_idx.min(end)..end] {
            let v1 = [t.v0[0] + t.e1[0], t.v0[1] + t.e1[1], t.v0[2] + t.e1[2]];
            let v2 = [t.v0[0] + t.e2[0], t.v0[1] + t.e2[1], t.v0[2] + t.e2[2]];
            include_point(&mut min, &mut max, transform_point(o, t.v0));
            include_point(&mut min, &mut max, transform_point(o, v1));
            include_point(&mut min, &mut max, transform_point(o, v2));
        }
    } else if o.is_cube {
        let half = [o.size[0] * 0.5, o.size[1] * 0.5, o.size[2] * 0.5];
        for &x in &[-half[0], half[0]] {
            for &y in &[-half[1], half[1]] {
                for &z in &[-half[2], half[2]] {
                    include_point(&mut min, &mut max, transform_point(o, [x, y, z]));
                }
            }
        }
    } else {
        for d in 0..3 {
            min[d] = o.position[d] - o.radius;
            max[d] = o.position[d] + o.radius;
        }
    }
    if min[0] == f32::MAX {
        min = o.position;
        max = o.position;
    }
    ObjBounds {
        index,
        min,
        max,
        center: [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ],
    }
}

fn union_bounds(entries: &[ObjBounds]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for e in entries {
        for d in 0..3 {
            min[d] = min[d].min(e.min[d]);
            max[d] = max[d].max(e.max[d]);
        }
    }
    (min, max)
}

pub fn build_bvh(objects: &[Object], triangles: &[GpuTriangle]) -> Vec<GpuBvhNode> {
    fn build(nodes: &mut Vec<GpuBvhNode>, entries: &mut [ObjBounds]) -> i32 {
        if entries.is_empty() {
            return -1;
        }
        let (min, max) = union_bounds(entries);
        let idx = nodes.len() as i32;
        nodes.push(GpuBvhNode {
            bounds_min: [min[0], min[1], min[2], 0.0],
            bounds_max: [max[0], max[1], max[2], 0.0],
            children: [-1, -1, -1, -1],
        });
        if entries.len() == 1 {
            nodes[idx as usize].children = [-1, -1, entries[0].index as i32, -1];
            return idx;
        }
        let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
            0
        } else if extent[1] >= extent[2] {
            1
        } else {
            2
        };
        let mid = entries.len() / 2;
        entries.select_nth_unstable_by(mid, |a, b| {
            a.center[axis]
                .partial_cmp(&b.center[axis])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let (left_entries, right_entries) = entries.split_at_mut(mid);
        let left = build(nodes, left_entries);
        let right = build(nodes, right_entries);
        nodes[idx as usize].children = [left, right, -1, 0];
        idx
    }

    let mut entries: Vec<_> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| object_bounds(o, triangles, i))
        .collect();
    let mut nodes = Vec::with_capacity(objects.len().saturating_mul(2).saturating_sub(1));
    build(&mut nodes, &mut entries);
    nodes
}
