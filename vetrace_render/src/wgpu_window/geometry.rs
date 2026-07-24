use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn object_geometry(object: &RenderObject, assets: Option<&RenderAssets>, extra: Option<f32>) -> Option<IndexedGeometry> {
    let extra = extra.unwrap_or(0.0);
    if let Some(mesh_handle) = object.mesh {
        if let Some(mesh) = assets.and_then(|assets| assets.meshes.get(&mesh_handle.0)) {
            return mesh_geometry(object, mesh, extra);
        }
    }
    let shape = object.shape.clone().unwrap_or(Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE });
    match shape.primitive {
        PrimitiveShape::Plane | PrimitiveShape::Quad => Some(plane_geometry(object, shape.size, extra)),
        PrimitiveShape::Cube => Some(cube_geometry(object, shape.size, extra)),
        PrimitiveShape::Sphere => Some(sphere_geometry(object, shape.size, extra, 48, 24)),
        PrimitiveShape::Capsule => Some(capsule_geometry(object, shape.size, extra, 24, 8)),
    }
}

pub(super) fn object_model_matrix(object: &RenderObject) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        object.transform.scale,
        object.transform.rotation,
        object.transform.translation,
    )
}

pub(super) fn transform_point(_object: &RenderObject, local: Vec3) -> Vec3 {
    // Vertices are kept in local space and transformed in WGSL by the per-object model matrix.
    local
}

pub(super) fn transform_normal(_object: &RenderObject, local: Vec3) -> Vec3 {
    if local.length_squared() > 1.0e-8 { local.normalize() } else { Vec3::Y }
}

pub(super) fn transform_tangent(_object: &RenderObject, local: Vec4) -> Vec4 {
    let xyz = Vec3::new(local.x, local.y, local.z);
    let tangent = if xyz.length_squared() > 1.0e-8 { xyz.normalize() } else { Vec3::X };
    Vec4::new(tangent.x, tangent.y, tangent.z, if local.w < 0.0 { -1.0 } else { 1.0 })
}

pub(super) fn safe_object_scale(object: &RenderObject) -> Vec3 {
    object.transform.scale.abs().max(Vec3::splat(0.001))
}

pub(super) fn axis_local_extra_for_world_extra(object: &RenderObject, axis: Vec3, extra: f32) -> f32 {
    if extra <= 0.0 {
        return 0.0;
    }
    let n = axis.normalize_or_zero();
    if n.length_squared() <= 1.0e-8 {
        return 0.0;
    }
    let s = safe_object_scale(object);
    let world_normal_len = Vec3::new(n.x * s.x, n.y * s.y, n.z * s.z).length().max(0.001);
    extra / world_normal_len
}

pub(super) fn offset_local_position_by_world_extra(object: &RenderObject, position: Vec3, normal: Vec3, extra: f32) -> Vec3 {
    let n = normal.normalize_or_zero();
    if extra <= 0.0 || n.length_squared() <= 1.0e-8 {
        return position;
    }
    position + n * axis_local_extra_for_world_extra(object, n, extra)
}

pub(super) fn v(object: &RenderObject, p: Vec3, n: Vec3) -> GpuVertex {
    GpuVertex {
        position: transform_point(object, p).to_array(),
        normal: transform_normal(object, n).to_array(),
        uv: [0.0, 0.0],
        lightmap_uv: [0.0, 0.0],
        color: [1.0, 1.0, 1.0, 1.0],
        tangent: transform_tangent(object, Vec4::new(1.0, 0.0, 0.0, 1.0)).to_array(),
    }
}


pub(super) fn skin_mesh_vertex(object: &RenderObject, vertex: &MeshVertex, normal_fallback: Vec3) -> (Vec3, Vec3, Vec4) {
    let mut normal = if vertex.normal.length_squared() > 1.0e-8 { vertex.normal.normalize() } else { normal_fallback };
    let tangent = vertex.tangent;
    let tangent_xyz = Vec3::new(tangent.x, tangent.y, tangent.z).normalize_or_zero();
    let Some(skin) = object.skin.as_ref() else {
        return (vertex.position, normal, vertex.tangent);
    };

    let mut weight_sum = 0.0_f32;
    let mut skinned_position = Vec3::ZERO;
    let mut skinned_normal = Vec3::ZERO;
    let mut skinned_tangent = Vec3::ZERO;
    for slot in 0..4 {
        let weight = vertex.weights[slot];
        if weight.abs() <= 1.0e-6 {
            continue;
        }
        let joint_index = vertex.joints[slot] as usize;
        let Some(matrix) = skin.joint_matrices.get(joint_index) else {
            continue;
        };
        weight_sum += weight;
        skinned_position += (*matrix * vertex.position.extend(1.0)).truncate() * weight;
        skinned_normal += (*matrix * normal.extend(0.0)).truncate() * weight;
        skinned_tangent += (*matrix * tangent_xyz.extend(0.0)).truncate() * weight;
    }

    if weight_sum <= 1.0e-6 {
        return (vertex.position, normal, vertex.tangent);
    }

    let inv_weight_sum = 1.0 / weight_sum;
    let position = skinned_position * inv_weight_sum;
    normal = (skinned_normal * inv_weight_sum).normalize_or_zero();
    if normal.length_squared() <= 1.0e-8 {
        normal = normal_fallback;
    }
    let tangent_xyz = (skinned_tangent * inv_weight_sum).normalize_or_zero();
    let tangent = if tangent_xyz.length_squared() > 1.0e-8 {
        Vec4::new(tangent_xyz.x, tangent_xyz.y, tangent_xyz.z, vertex.tangent.w)
    } else {
        vertex.tangent
    };
    (position, normal, tangent)
}

pub(super) fn mesh_geometry(object: &RenderObject, mesh: &MeshAsset, extra: f32) -> Option<IndexedGeometry> {
    if mesh.vertices.is_empty() { return None; }
    let normal_fallback = Vec3::Y;
    let push_vertex = |out: &mut Vec<GpuVertex>, vertex: &MeshVertex| {
        let (position, normal, tangent) = skin_mesh_vertex(object, vertex, normal_fallback);
        let local_normal = if normal.length_squared() > 1.0e-8 { normal.normalize() } else { normal_fallback };
        let local_position = offset_local_position_by_world_extra(object, position, local_normal, extra);
        let world_normal = transform_normal(object, local_normal);
        let color = vertex.color.clamp(Vec4::ZERO, Vec4::ONE);
        let world_tangent = transform_tangent(object, tangent);
        out.push(GpuVertex {
            position: transform_point(object, local_position).to_array(),
            normal: world_normal.to_array(),
            uv: vertex.uv.to_array(),
            lightmap_uv: vertex.lightmap_uv.to_array(),
            color: color.to_array(),
            tangent: world_tangent.to_array(),
        });
    };

    if mesh.indices.is_empty() {
        let mut out = Vec::with_capacity(mesh.vertices.len());
        for vertex in &mesh.vertices {
            push_vertex(&mut out, vertex);
        }
        return Some(IndexedGeometry { vertices: out, indices: None });
    }

    for index in &mesh.indices {
        if (*index as usize) >= mesh.vertices.len() {
            return None;
        }
    }
    let mut out = Vec::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        push_vertex(&mut out, vertex);
    }
    Some(IndexedGeometry { vertices: out, indices: Some(mesh.indices.clone()) })
}

pub(super) fn cube_geometry(object: &RenderObject, size: Vec3, extra: f32) -> IndexedGeometry {
    let base_half = (if size.length_squared() > 0.0 { size } else { Vec3::ONE }).abs() * 0.5;
    let half = base_half + Vec3::new(
        axis_local_extra_for_world_extra(object, Vec3::X, extra),
        axis_local_extra_for_world_extra(object, Vec3::Y, extra),
        axis_local_extra_for_world_extra(object, Vec3::Z, extra),
    );
    let p = [
        Vec3::new(-half.x, -half.y, -half.z),
        Vec3::new( half.x, -half.y, -half.z),
        Vec3::new( half.x,  half.y, -half.z),
        Vec3::new(-half.x,  half.y, -half.z),
        Vec3::new(-half.x, -half.y,  half.z),
        Vec3::new( half.x, -half.y,  half.z),
        Vec3::new( half.x,  half.y,  half.z),
        Vec3::new(-half.x,  half.y,  half.z),
    ];
    let faces = [
        ([0, 3, 2, 1], Vec3::NEG_Z),
        ([4, 5, 6, 7], Vec3::Z),
        ([0, 4, 7, 3], Vec3::NEG_X),
        ([1, 2, 6, 5], Vec3::X),
        ([3, 7, 6, 2], Vec3::Y),
        ([0, 1, 5, 4], Vec3::NEG_Y),
    ];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (face_index, (idx, normal)) in faces.into_iter().enumerate() {
        let base = (face_index * 4) as u32;
        let corners = [p[idx[0]], p[idx[1]], p[idx[2]], p[idx[3]]];
        let face_uv_size = cube_face_uv_size_from_order(object, corners);
        let mut face_vertices = [
            v(object, corners[0], normal),
            v(object, corners[1], normal),
            v(object, corners[2], normal),
            v(object, corners[3], normal),
        ];
        // Use the actual two edge lengths for this face/order instead of only
        // picking dimensions from the face normal. Some cube faces need reversed
        // winding for correct outward normals; using the normal alone swaps U/V
        // on those faces and stretches rectangular textures. This keeps every
        // box side world-size tiled even when the cube is scaled into a wall.
        face_vertices[0].uv = [0.0, 0.0];
        face_vertices[1].uv = [0.0, face_uv_size.y];
        face_vertices[2].uv = [face_uv_size.x, face_uv_size.y];
        face_vertices[3].uv = [face_uv_size.x, 0.0];
        let lightmap_uv = crate::baked_lighting::cube_face_lightmap_uv(face_index);
        for (vertex, uv) in face_vertices.iter_mut().zip(lightmap_uv) {
            vertex.lightmap_uv = uv.to_array();
        }
        vertices.extend_from_slice(&face_vertices);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    IndexedGeometry { vertices, indices: Some(indices) }
}

pub(super) fn cube_face_uv_size_from_order(object: &RenderObject, corners: [Vec3; 4]) -> Vec2 {
    let scale = safe_object_scale(object);
    let scaled_edge_len = |a: Vec3, b: Vec3| -> f32 {
        ((b - a) * scale).length().abs().max(0.001)
    };
    // With the UV assignment below, V runs along corner 0 -> 1 and U runs
    // along corner 1 -> 2. Measuring those exact edges makes the mapping
    // independent of face winding and avoids stretched cube sides.
    Vec2::new(
        scaled_edge_len(corners[1], corners[2]),
        scaled_edge_len(corners[0], corners[1]),
    )
}

pub(super) fn plane_geometry(object: &RenderObject, size: Vec3, extra: f32) -> IndexedGeometry {
    let sx = (if size.x.abs() > 0.0 { size.x.abs() } else { 1.0 }) * 0.5
        + axis_local_extra_for_world_extra(object, Vec3::X, extra);
    let sz = (if size.z.abs() > 0.0 { size.z.abs() } else { size.y.abs().max(1.0) }) * 0.5
        + axis_local_extra_for_world_extra(object, Vec3::Z, extra);
    let y = 0.0;
    let n = Vec3::Y;
    let a = Vec3::new(-sx, y, -sz);
    let b = Vec3::new( sx, y, -sz);
    let c = Vec3::new( sx, y,  sz);
    let d = Vec3::new(-sx, y,  sz);
    let object_scale = safe_object_scale(object);
    let u = (sx * 2.0 * object_scale.x).abs().max(0.001);
    let v_len = (sz * 2.0 * object_scale.z).abs().max(0.001);
    let mut vertices = vec![v(object, a, n), v(object, b, n), v(object, c, n), v(object, d, n)];
    vertices[0].uv = [0.0, 0.0];
    vertices[1].uv = [u, 0.0];
    vertices[2].uv = [u, v_len];
    vertices[3].uv = [0.0, v_len];
    vertices[0].lightmap_uv = [0.0, 0.0];
    vertices[1].lightmap_uv = [1.0, 0.0];
    vertices[2].lightmap_uv = [1.0, 1.0];
    vertices[3].lightmap_uv = [0.0, 1.0];
    IndexedGeometry {
        vertices,
        // WGPU uses CCW front faces. From above the plane (+Y), these triangles
        // must wind counter-clockwise or the default back-face culling path will
        // drop them and double-sided materials will get flipped lighting.
        indices: Some(vec![0, 3, 2, 0, 2, 1]),
    }
}


pub(super) fn sphere_geometry(object: &RenderObject, size: Vec3, extra: f32, sectors: usize, stacks: usize) -> IndexedGeometry {
    let sectors = sectors.max(8);
    let stacks = stacks.max(4);
    let radius = size.max_element().abs().max(0.05) * 0.5;
    let mut vertices = Vec::with_capacity((stacks + 1) * (sectors + 1));
    let mut indices = Vec::with_capacity(stacks * sectors * 6);

    for stack in 0..=stacks {
        let v01 = stack as f32 / stacks as f32;
        let phi = std::f32::consts::FRAC_PI_2 - v01 * std::f32::consts::PI;
        let y = phi.sin();
        let ring = phi.cos();
        for sector in 0..=sectors {
            let u01 = sector as f32 / sectors as f32;
            let theta = u01 * std::f32::consts::TAU;
            let normal = Vec3::new(ring * theta.cos(), y, ring * theta.sin()).normalize_or_zero();
            let mut vertex = v(object, offset_local_position_by_world_extra(object, normal * radius, normal, extra), normal);
            let uv_radius = radius * safe_object_scale(object).max_element();
            vertex.uv = [u01 * uv_radius * std::f32::consts::TAU, v01 * uv_radius * std::f32::consts::PI];
            vertex.lightmap_uv = [u01, v01];
            vertices.push(vertex);
        }
    }

    let stride = sectors + 1;
    for stack in 0..stacks {
        for sector in 0..sectors {
            let a = (stack * stride + sector) as u32;
            let b = ((stack + 1) * stride + sector) as u32;
            let c = ((stack + 1) * stride + sector + 1) as u32;
            let d = (stack * stride + sector + 1) as u32;
            if stack != 0 {
                indices.extend_from_slice(&[a, d, b]);
            }
            if stack + 1 != stacks {
                indices.extend_from_slice(&[d, c, b]);
            }
        }
    }

    IndexedGeometry { vertices, indices: Some(indices) }
}

pub(super) fn capsule_geometry(object: &RenderObject, size: Vec3, extra: f32, sectors: usize, hemisphere_rings: usize) -> IndexedGeometry {
    let sectors = sectors.max(8);
    let hemisphere_rings = hemisphere_rings.max(3);
    let radius = (size.x.abs().max(size.z.abs()) * 0.5).max(0.05);
    let total_height = size.y.abs().max(radius * 2.0 + 0.05);
    let cylinder_half = (total_height * 0.5 - radius).max(0.0);

    let mut rings: Vec<(f32, f32)> = Vec::new();
    // Top pole to equator.
    for i in 0..=hemisphere_rings {
        let t = i as f32 / hemisphere_rings as f32;
        let phi = std::f32::consts::FRAC_PI_2 * (1.0 - t);
        rings.push((cylinder_half + radius * phi.sin(), phi.cos()));
    }
    // Bottom equator to pole. Skip duplicated equator.
    for i in 1..=hemisphere_rings {
        let t = i as f32 / hemisphere_rings as f32;
        let phi = -std::f32::consts::FRAC_PI_2 * t;
        rings.push((-cylinder_half + radius * phi.sin(), phi.cos()));
    }

    let mut vertices = Vec::with_capacity(rings.len() * (sectors + 1));
    let mut indices = Vec::with_capacity((rings.len().saturating_sub(1)) * sectors * 6);
    let ring_count = rings.len();

    for (ring_index, (y, ring_radius_factor)) in rings.iter().copied().enumerate() {
        let v01 = ring_index as f32 / (ring_count.saturating_sub(1).max(1)) as f32;
        let center_y = if y >= 0.0 { cylinder_half } else { -cylinder_half };
        for sector in 0..=sectors {
            let u01 = sector as f32 / sectors as f32;
            let theta = u01 * std::f32::consts::TAU;
            let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
            let xz = radial * (ring_radius_factor * radius);
            let pos = Vec3::new(xz.x, y, xz.z);
            let normal = Vec3::new(xz.x, y - center_y, xz.z).normalize_or_zero();
            let normal = if normal.length_squared() > 1.0e-8 { normal } else { Vec3::Y };
            let mut vertex = v(object, offset_local_position_by_world_extra(object, pos, normal, extra), normal);
            let object_scale = safe_object_scale(object);
            let uv_radius = radius * object_scale.x.max(object_scale.z).max(0.001);
            let uv_height = total_height.max(radius * 2.0) * object_scale.y.max(0.001);
            vertex.uv = [u01 * uv_radius * std::f32::consts::TAU, v01 * uv_height];
            vertex.lightmap_uv = [u01, v01];
            vertices.push(vertex);
        }
    }

    let stride = sectors + 1;
    for ring in 0..ring_count.saturating_sub(1) {
        for sector in 0..sectors {
            let a = (ring * stride + sector) as u32;
            let b = ((ring + 1) * stride + sector) as u32;
            let c = ((ring + 1) * stride + sector + 1) as u32;
            let d = (ring * stride + sector + 1) as u32;
            indices.extend_from_slice(&[a, d, b, d, c, b]);
        }
    }

    IndexedGeometry { vertices, indices: Some(indices) }
}
