use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CollisionIntent {
    pub shape: GltfCollisionShapeKind,
    pub body: GltfCollisionBodyKind,
    pub sensor: bool,
    pub hide_render_mesh: bool,
}

pub(crate) fn named_collision_intent(name: &str, options: &GltfLoadOptions) -> Option<CollisionIntent> {
    if !options.import_collisions || !options.collision_policy.uses_named_nodes() {
        return None;
    }

    let lower = normalized_name(name);
    let sensor = lower.starts_with("trigger") || lower.starts_with("trg") || lower.contains("_trigger_");
    let is_collision = sensor
        || lower == "col"
        || lower.starts_with("col_")
        || lower.starts_with("collider")
        || lower.starts_with("collision")
        || lower.starts_with("ucx")
        || lower.starts_with("ubx")
        || lower.starts_with("usp")
        || lower.starts_with("ucp");
    if !is_collision {
        return None;
    }

    let shape = if lower.starts_with("ubx") || contains_token(&lower, "box") || contains_token(&lower, "cube") {
        GltfCollisionShapeKind::Box
    } else if lower.starts_with("usp") || contains_token(&lower, "sphere") || contains_token(&lower, "ball") {
        GltfCollisionShapeKind::Sphere
    } else if lower.starts_with("ucp") || contains_token(&lower, "capsule") {
        GltfCollisionShapeKind::Capsule
    } else if lower.starts_with("ucx") || contains_token(&lower, "convex") || contains_token(&lower, "hull") {
        GltfCollisionShapeKind::ConvexHull
    } else if contains_token(&lower, "mesh") || contains_token(&lower, "trimesh") || contains_token(&lower, "tri") {
        GltfCollisionShapeKind::TriangleMesh
    } else {
        // For map helper meshes named simply `COL_floor`/`COL_wall`, a static
        // triangle mesh is the most useful default. Empty marker nodes still
        // fall back to a primitive box in `empty_collider_from_intent`.
        GltfCollisionShapeKind::TriangleMesh
    };

    let body = if lower.starts_with("dyn") || contains_token(&lower, "dynamic") || contains_token(&lower, "rigid") {
        GltfCollisionBodyKind::Dynamic
    } else if lower.starts_with("kin") || contains_token(&lower, "kinematic") {
        GltfCollisionBodyKind::Kinematic
    } else {
        GltfCollisionBodyKind::Static
    };

    Some(CollisionIntent { shape, body, sensor, hide_render_mesh: true })
}

pub(crate) fn should_auto_static_mesh(options: &GltfLoadOptions, named_intent: Option<CollisionIntent>) -> bool {
    options.import_collisions && options.collision_policy.uses_auto_static_mesh() && named_intent.is_none()
}

pub(crate) fn collider_from_mesh_asset(
    name: &str,
    mesh: &MeshAsset,
    intent: CollisionIntent,
) -> Option<GltfImportedCollider> {
    let vertices: Vec<Vec3> = mesh.vertices.iter().map(|vertex| vertex.position).collect();
    if vertices.is_empty() {
        return None;
    }
    let indices = triangles_from_indices(&mesh.indices, vertices.len());
    let (center, half_extents) = bounds_center_half_extents(&vertices);
    let mut shape = intent.shape;
    if matches!(shape, GltfCollisionShapeKind::TriangleMesh) && indices.is_empty() {
        // A triangle-mesh collider with no triangles is useless. Convex hull can
        // still be generated from point clouds, and primitives can use bounds.
        shape = GltfCollisionShapeKind::ConvexHull;
    }

    Some(GltfImportedCollider {
        body: intent.body,
        shape,
        sensor: intent.sensor,
        half_extents,
        offset: if uses_mesh_points(shape) { Vec3::ZERO } else { center },
        vertices,
        indices,
        source: name.to_string(),
    })
}

pub(crate) fn auto_static_collider_from_mesh_asset(name: &str, mesh: &MeshAsset) -> Option<GltfImportedCollider> {
    collider_from_mesh_asset(
        name,
        mesh,
        CollisionIntent {
            shape: GltfCollisionShapeKind::TriangleMesh,
            body: GltfCollisionBodyKind::Static,
            sensor: false,
            hide_render_mesh: false,
        },
    )
}

pub(crate) fn empty_collider_from_intent(name: &str, intent: CollisionIntent) -> GltfImportedCollider {
    let shape = match intent.shape {
        GltfCollisionShapeKind::ConvexHull | GltfCollisionShapeKind::TriangleMesh => GltfCollisionShapeKind::Box,
        shape => shape,
    };
    GltfImportedCollider {
        body: intent.body,
        shape,
        sensor: intent.sensor,
        half_extents: Vec3::splat(0.5),
        offset: Vec3::ZERO,
        vertices: Vec::new(),
        indices: Vec::new(),
        source: name.to_string(),
    }
}

pub(crate) fn mark_collision_entity(engine: &mut Engine, entity: Entity, source: &str) {
    let Some(actor) = engine.actor(entity) else { return; };
    let _ = actor.add_tag(engine, "gltf_collision");
    if actor.source(engine).is_none() {
        let _ = actor.set_source(engine, Some(source.to_string()));
    }
}

fn normalized_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' })
        .collect()
}

fn contains_token(text: &str, token: &str) -> bool {
    text.split('_').any(|part| part == token) || text.contains(&format!("_{token}_"))
}

fn uses_mesh_points(shape: GltfCollisionShapeKind) -> bool {
    matches!(shape, GltfCollisionShapeKind::ConvexHull | GltfCollisionShapeKind::TriangleMesh)
}

fn bounds_center_half_extents(vertices: &[Vec3]) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for vertex in vertices {
        min = min.min(*vertex);
        max = max.max(*vertex);
    }
    if !min.is_finite() || !max.is_finite() {
        return (Vec3::ZERO, Vec3::splat(0.5));
    }
    let center = (min + max) * 0.5;
    let half_extents = ((max - min) * 0.5).abs().max(Vec3::splat(0.001));
    (center, half_extents)
}

fn triangles_from_indices(indices: &[u32], vertex_count: usize) -> Vec<[u32; 3]> {
    if indices.is_empty() {
        return (0..vertex_count as u32)
            .collect::<Vec<_>>()
            .chunks_exact(3)
            .map(|tri| [tri[0], tri[1], tri[2]])
            .collect();
    }
    indices
        .chunks_exact(3)
        .filter_map(|tri| {
            let a = tri[0] as usize;
            let b = tri[1] as usize;
            let c = tri[2] as usize;
            (a < vertex_count && b < vertex_count && c < vertex_count).then_some([tri[0], tri[1], tri[2]])
        })
        .collect()
}
