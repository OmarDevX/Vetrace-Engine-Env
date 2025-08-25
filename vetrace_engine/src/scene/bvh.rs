use crate::scene::object::{GpuTriangle, Object};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct GpuBvhNode {
    pub center_radius: [f32; 4],
    pub children: [i32; 4],
}

pub fn build_bvh(objects: &[Object], _triangles: &[GpuTriangle]) -> Vec<GpuBvhNode> {
    fn obj_radius(o: &Object) -> f32 {
        if o.is_mesh || o.is_cube {
            // Combine the object's intrinsic size with its scale so the
            // bounding sphere fully encloses the geometry in world space.
            let sx = o.size[0] * o.scale[0];
            let sy = o.size[1] * o.scale[1];
            let sz = o.size[2] * o.scale[2];
            let r2 = sx * sx + sy * sy + sz * sz;
            (r2).sqrt() * 0.5
        } else {
            o.radius
        }
    }

    fn bounding_sphere(objects: &[Object], indices: &[usize]) -> ([f32; 3], f32) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for &i in indices {
            let o = &objects[i];
            let r = obj_radius(o);
            for d in 0..3 {
                let p = o.position[d];
                if p - r < min[d] {
                    min[d] = p - r;
                }
                if p + r > max[d] {
                    max[d] = p + r;
                }
            }
        }
        let center = [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ];

        let mut radius = 0.0;
        for &i in indices {
            let o = &objects[i];
            let r = obj_radius(o);
            let dx = o.position[0] - center[0];
            let dy = o.position[1] - center[1];
            let dz = o.position[2] - center[2];
            let dist = (dx * dx + dy * dy + dz * dz).sqrt() + r;
            if dist > radius {
                radius = dist;
            }
        }
        (center, radius)
    }

    fn build(nodes: &mut Vec<GpuBvhNode>, objects: &[Object], indices: &[usize]) -> i32 {
        if indices.is_empty() {
            return -1;
        }
        if indices.len() == 1 {
            let i = indices[0];
            let r = obj_radius(&objects[i]);
            let idx = nodes.len() as i32;
            nodes.push(GpuBvhNode {
                center_radius: [
                    objects[i].position[0],
                    objects[i].position[1],
                    objects[i].position[2],
                    r,
                ],
                // Use -1 to indicate the absence of a second object in the leaf.
                children: [-1, -1, i as i32, -1],
            });
            return idx;
        }

        let (center, radius) = bounding_sphere(objects, indices);
        let idx = nodes.len() as i32;
        nodes.push(GpuBvhNode {
            center_radius: [center[0], center[1], center[2], radius],
            // Initialize leaf object slots to -1 to avoid accidental indices.
            children: [-1, -1, -1, -1],
        });

        let mut inds = indices.to_vec();
        // Determine longest axis of bounding box for split
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for &i in indices {
            for d in 0..3 {
                let p = objects[i].position[d];
                if p < min[d] {
                    min[d] = p;
                }
                if p > max[d] {
                    max[d] = p;
                }
            }
        }
        let extent = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        let axis = if extent[0] >= extent[1] && extent[0] >= extent[2] {
            0
        } else if extent[1] >= extent[0] && extent[1] >= extent[2] {
            1
        } else {
            2
        };
        inds.sort_by(|&a, &b| objects[a].position[axis].partial_cmp(&objects[b].position[axis]).unwrap());
        let mid = inds.len() / 2;
        let left = build(nodes, objects, &inds[..mid]);
        let right = build(nodes, objects, &inds[mid..]);
        nodes[idx as usize].children = [left, right, -1, 0];
        idx
    }

    let mut nodes = Vec::new();
    let indices: Vec<usize> = (0..objects.len()).collect();
    if !indices.is_empty() {
        build(&mut nodes, objects, &indices);
    }
    nodes
}