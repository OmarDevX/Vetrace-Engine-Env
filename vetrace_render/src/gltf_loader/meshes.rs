use super::*;

pub(crate) fn mesh_asset_from_primitive(
    name: &str,
    primitive: &gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    generate_missing_normals: bool,
) -> Result<MeshAsset> {
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &**data));
    let positions: Vec<Vec3> = reader
        .read_positions()
        .context("glTF primitive is missing POSITION attribute")?
        .map(Vec3::from_array)
        .collect();

    if positions.is_empty() {
        return Ok(MeshAsset { name: name.to_string(), vertices: Vec::new(), indices: Vec::new(), revision: 0 });
    }

    let indices: Vec<u32> = reader
        .read_indices()
        .map(|indices| indices.into_u32().collect())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());

    let mut normals: Vec<Vec3> = reader
        .read_normals()
        .map(|normals| normals.map(Vec3::from_array).collect())
        .unwrap_or_default();
    if normals.len() != positions.len() {
        normals = if generate_missing_normals {
            generate_normals(&positions, &indices)
        } else {
            vec![Vec3::Y; positions.len()]
        };
    }

    let texcoords: Vec<Vec2> = reader
        .read_tex_coords(0)
        .map(|uvs| uvs.into_f32().map(Vec2::from_array).collect())
        .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);

    let lightmap_texcoords: Vec<Vec2> = reader
        .read_tex_coords(1)
        .map(|uvs| uvs.into_f32().map(Vec2::from_array).collect())
        .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);

    let mut tangents: Vec<Vec4> = reader
        .read_tangents()
        .map(|tangents| tangents.map(Vec4::from_array).collect())
        .unwrap_or_default();
    if tangents.len() != positions.len() {
        tangents = generate_tangents(&positions, &normals, &texcoords, &indices);
    }

    let colors: Vec<Vec4> = reader
        .read_colors(0)
        .map(|colors| colors.into_rgba_f32().map(Vec4::from_array).collect())
        .unwrap_or_else(|| vec![Vec4::ONE; positions.len()]);

    let joint_indices: Vec<[u16; 4]> = reader
        .read_joints(0)
        .map(|joints| joints.into_u16().collect())
        .unwrap_or_else(|| vec![[0; 4]; positions.len()]);

    let joint_weights: Vec<[f32; 4]> = reader
        .read_weights(0)
        .map(|weights| weights.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0; 4]; positions.len()]);

    let mut vertices = Vec::with_capacity(positions.len());
    for index in 0..positions.len() {
        vertices.push(MeshVertex {
            position: positions[index],
            normal: normals.get(index).copied().unwrap_or(Vec3::Y).normalize_or_zero(),
            uv: texcoords.get(index).copied().unwrap_or(Vec2::ZERO),
            lightmap_uv: lightmap_texcoords.get(index).copied().unwrap_or(Vec2::ZERO),
            tangent: tangents.get(index).copied().unwrap_or(Vec4::new(1.0, 0.0, 0.0, 1.0)),
            color: colors.get(index).copied().unwrap_or(Vec4::ONE),
            joints: joint_indices.get(index).copied().unwrap_or([0; 4]),
            weights: joint_weights.get(index).copied().unwrap_or([0.0; 4]),
        });
    }

    Ok(MeshAsset { name: name.to_string(), vertices, indices, revision: 0 })
}

fn generate_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks_exact(3) {
        let Some(a) = positions.get(tri[0] as usize).copied() else { continue; };
        let Some(b) = positions.get(tri[1] as usize).copied() else { continue; };
        let Some(c) = positions.get(tri[2] as usize).copied() else { continue; };
        let face = (b - a).cross(c - a).normalize_or_zero();
        if face.length_squared() <= 1.0e-8 { continue; }
        for index in tri {
            if let Some(normal) = normals.get_mut(*index as usize) {
                *normal += face;
            }
        }
    }

    for normal in &mut normals {
        *normal = if normal.length_squared() > 1.0e-8 { normal.normalize() } else { Vec3::Y };
    }
    normals
}

fn generate_tangents(positions: &[Vec3], normals: &[Vec3], texcoords: &[Vec2], indices: &[u32]) -> Vec<Vec4> {
    if positions.is_empty() || texcoords.len() != positions.len() || normals.len() != positions.len() {
        return vec![Vec4::new(1.0, 0.0, 0.0, 1.0); positions.len()];
    }

    let mut tan1 = vec![Vec3::ZERO; positions.len()];
    let mut tan2 = vec![Vec3::ZERO; positions.len()];

    for tri in indices.chunks_exact(3) {
        let i1 = tri[0] as usize;
        let i2 = tri[1] as usize;
        let i3 = tri[2] as usize;
        let (Some(p1), Some(p2), Some(p3)) = (positions.get(i1), positions.get(i2), positions.get(i3)) else { continue; };
        let (Some(w1), Some(w2), Some(w3)) = (texcoords.get(i1), texcoords.get(i2), texcoords.get(i3)) else { continue; };

        let x1 = p2.x - p1.x;
        let x2 = p3.x - p1.x;
        let y1 = p2.y - p1.y;
        let y2 = p3.y - p1.y;
        let z1 = p2.z - p1.z;
        let z2 = p3.z - p1.z;
        let s1 = w2.x - w1.x;
        let s2 = w3.x - w1.x;
        let t1 = w2.y - w1.y;
        let t2 = w3.y - w1.y;
        let denom = s1 * t2 - s2 * t1;
        if denom.abs() <= 1.0e-8 { continue; }
        let r = 1.0 / denom;
        let sdir = Vec3::new((t2 * x1 - t1 * x2) * r, (t2 * y1 - t1 * y2) * r, (t2 * z1 - t1 * z2) * r);
        let tdir = Vec3::new((s1 * x2 - s2 * x1) * r, (s1 * y2 - s2 * y1) * r, (s1 * z2 - s2 * z1) * r);

        for i in [i1, i2, i3] {
            if let Some(t) = tan1.get_mut(i) { *t += sdir; }
            if let Some(t) = tan2.get_mut(i) { *t += tdir; }
        }
    }

    let mut out = Vec::with_capacity(positions.len());
    for i in 0..positions.len() {
        let n = normals.get(i).copied().unwrap_or(Vec3::Y).normalize_or_zero();
        let t = tan1.get(i).copied().unwrap_or(Vec3::X);
        let tangent = (t - n * n.dot(t)).normalize_or_zero();
        if tangent.length_squared() <= 1.0e-8 {
            out.push(Vec4::new(1.0, 0.0, 0.0, 1.0));
            continue;
        }
        let handedness = if n.cross(t).dot(tan2.get(i).copied().unwrap_or(Vec3::Y)) < 0.0 { -1.0 } else { 1.0 };
        out.push(Vec4::new(tangent.x, tangent.y, tangent.z, handedness));
    }
    out
}
