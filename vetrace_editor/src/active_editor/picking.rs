use super::*;
use vetrace_render::{MeshAsset, RenderAssets};

const RAY_EPSILON: f32 = 1.0e-6;

pub(crate) fn entity_label(engine: &Engine, entity: Entity) -> Option<String> {
    engine.raw_world().get::<Name>(entity)
        .map(|name| name.0.clone())
        .or_else(|| engine.raw_world().is_alive(entity).then(|| format!("Entity({})", entity.0)))
}

pub(crate) fn pick_entity_from_mouse(engine: &Engine, mouse: (f32, f32)) -> Option<(Entity, f32)> {
    #[cfg(feature = "render_2d")]
    if engine
        .get_resource::<EditorState>()
        .map(|state| state.viewport_mode == EditorViewportMode::TwoD)
        .unwrap_or(false)
    {
        return super::picking_2d::pick_entity_2d_from_mouse(engine, mouse);
    }
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let camera = engine.get_resource::<Camera>().cloned().unwrap_or_default();
    let (origin, dir) = mouse_ray(&camera, &settings, Vec2::new(mouse.0, mouse.1))?;
    let assets = engine.get_resource::<RenderAssets>();
    let mut best: Option<(Entity, f32)> = None;

    for entity in selectable_entities(engine) {
        let transform = global_transform_for(engine, entity);
        let distance = entity_intersection(engine, assets, entity, origin, dir, &transform);
        if let Some(distance) = distance {
            if distance > RAY_EPSILON && best.map(|(_, current)| distance < current).unwrap_or(true) {
                best = Some((entity, distance));
            }
        }
    }
    best
}

fn entity_intersection(
    engine: &Engine,
    assets: Option<&RenderAssets>,
    entity: Entity,
    origin: Vec3,
    dir: Vec3,
    transform: &GlobalTransform,
) -> Option<f32> {
    if let Some(mesh) = engine
        .raw_world()
        .get::<Renderable>(entity)
        .and_then(|renderable| renderable.mesh)
        .and_then(|handle| assets.and_then(|assets| assets.meshes.get(&handle.0)))
    {
        return mesh_intersection(origin, dir, transform, mesh);
    }

    if let Some(shape) = engine.raw_world().get::<Shape>(entity) {
        return primitive_intersection(origin, dir, transform, shape);
    }

    // Unknown/custom renderables can still participate in selection, but the
    // broad fallback is deliberately last so a large ground primitive cannot
    // mask the actual object under the pointer.
    let radius = primitive_radius_for(engine, entity, transform);
    sphere_intersection(origin, dir, transform.translation, radius)
}

pub(crate) fn mouse_ray(camera: &Camera, settings: &RenderSettings, mouse: Vec2) -> Option<(Vec3, Vec3)> {
    let width = settings.width.max(1) as f32;
    let height = settings.height.max(1) as f32;
    let ndc_x = (mouse.x / width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (mouse.y / height) * 2.0;
    let forward = (camera.target - camera.position).normalize_or_zero();
    if forward.length_squared() <= f32::EPSILON { return None; }
    let right = forward.cross(camera.up).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    let tan_half = (camera.fov_y_radians * 0.5).tan();
    let aspect = width / height;
    let dir = (forward + right * ndc_x * tan_half * aspect + up * ndc_y * tan_half).normalize_or_zero();
    Some((camera.position, dir))
}

pub(crate) fn global_transform_for(engine: &Engine, entity: Entity) -> GlobalTransform {
    if let Some(global) = engine.raw_world().get::<GlobalTransform>(entity) {
        global.clone()
    } else if let Some(transform) = engine.raw_world().get::<Transform>(entity) {
        GlobalTransform::from(transform)
    } else {
        GlobalTransform::default()
    }
}

pub(crate) fn primitive_radius_for(engine: &Engine, entity: Entity, transform: &GlobalTransform) -> f32 {
    let shape = engine.raw_world().get::<Shape>(entity);
    let base = match shape.map(|shape| shape.primitive).unwrap_or(PrimitiveShape::Cube) {
        PrimitiveShape::Cube => shape.map(|shape| shape.size.length() * 0.5).unwrap_or(0.866),
        PrimitiveShape::Sphere => shape.map(|shape| shape.size.max_element().abs() * 0.5).unwrap_or(0.5),
        PrimitiveShape::Capsule => shape.map(|shape| shape.size.max_element().abs() * 0.5).unwrap_or(0.5),
        PrimitiveShape::Plane | PrimitiveShape::Quad => shape.map(|shape| shape.size.truncate().length() * 0.5).unwrap_or(0.707),
    };
    base * transform.scale.abs().max_element().max(0.01)
}

fn primitive_intersection(
    origin: Vec3,
    dir: Vec3,
    transform: &GlobalTransform,
    shape: &Shape,
) -> Option<f32> {
    let (local_origin, local_dir) = ray_to_local(origin, dir, transform)?;
    match shape.primitive {
        PrimitiveShape::Cube => {
            let half = shape.size.abs().max(Vec3::splat(0.001)) * 0.5;
            aabb_intersection(local_origin, local_dir, -half, half)
        }
        PrimitiveShape::Sphere => {
            let radius = shape.size.abs().max_element().max(0.05) * 0.5;
            sphere_intersection(local_origin, local_dir, Vec3::ZERO, radius)
        }
        PrimitiveShape::Capsule => capsule_intersection(local_origin, local_dir, shape.size),
        PrimitiveShape::Plane | PrimitiveShape::Quad => {
            let half_x = shape.size.x.abs().max(0.001) * 0.5;
            let half_z = shape.size.z.abs().max(shape.size.y.abs()).max(0.001) * 0.5;
            plane_intersection(local_origin, local_dir, half_x, half_z)
        }
    }
}

fn mesh_intersection(
    origin: Vec3,
    dir: Vec3,
    transform: &GlobalTransform,
    mesh: &MeshAsset,
) -> Option<f32> {
    let (local_origin, local_dir) = ray_to_local(origin, dir, transform)?;
    if mesh.vertices.len() < 3 {
        return None;
    }

    let mut best = f32::INFINITY;
    if mesh.indices.is_empty() {
        for triangle in mesh.vertices.chunks_exact(3) {
            if let Some(distance) = triangle_intersection(
                local_origin,
                local_dir,
                triangle[0].position,
                triangle[1].position,
                triangle[2].position,
            ) {
                best = best.min(distance);
            }
        }
    } else {
        for triangle in mesh.indices.chunks_exact(3) {
            let Some(a) = mesh.vertices.get(triangle[0] as usize) else { continue; };
            let Some(b) = mesh.vertices.get(triangle[1] as usize) else { continue; };
            let Some(c) = mesh.vertices.get(triangle[2] as usize) else { continue; };
            if let Some(distance) = triangle_intersection(
                local_origin,
                local_dir,
                a.position,
                b.position,
                c.position,
            ) {
                best = best.min(distance);
            }
        }
    }
    best.is_finite().then_some(best)
}

fn ray_to_local(origin: Vec3, dir: Vec3, transform: &GlobalTransform) -> Option<(Vec3, Vec3)> {
    let scale = transform.scale;
    if scale.x.abs() <= RAY_EPSILON || scale.y.abs() <= RAY_EPSILON || scale.z.abs() <= RAY_EPSILON {
        return None;
    }
    let inverse_rotation = transform.rotation.conjugate();
    let local_origin = (inverse_rotation * (origin - transform.translation)) / scale;
    let local_dir = (inverse_rotation * dir) / scale;
    Some((local_origin, local_dir))
}

fn aabb_intersection(origin: Vec3, dir: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut near = 0.0_f32;
    let mut far = f32::INFINITY;
    for (origin_axis, dir_axis, min_axis, max_axis) in [
        (origin.x, dir.x, min.x, max.x),
        (origin.y, dir.y, min.y, max.y),
        (origin.z, dir.z, min.z, max.z),
    ] {
        if dir_axis.abs() <= RAY_EPSILON {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }
        let inverse = 1.0 / dir_axis;
        let mut first = (min_axis - origin_axis) * inverse;
        let mut second = (max_axis - origin_axis) * inverse;
        if first > second {
            std::mem::swap(&mut first, &mut second);
        }
        near = near.max(first);
        far = far.min(second);
        if near > far {
            return None;
        }
    }
    if near > RAY_EPSILON {
        Some(near)
    } else if far > RAY_EPSILON {
        Some(far)
    } else {
        None
    }
}

fn plane_intersection(origin: Vec3, dir: Vec3, half_x: f32, half_z: f32) -> Option<f32> {
    if dir.y.abs() <= RAY_EPSILON {
        return None;
    }
    let distance = -origin.y / dir.y;
    if distance <= RAY_EPSILON {
        return None;
    }
    let point = origin + dir * distance;
    (point.x.abs() <= half_x + RAY_EPSILON && point.z.abs() <= half_z + RAY_EPSILON)
        .then_some(distance)
}

fn capsule_intersection(origin: Vec3, dir: Vec3, size: Vec3) -> Option<f32> {
    let radius = (size.x.abs().max(size.z.abs()) * 0.5).max(0.05);
    let total_height = size.y.abs().max(radius * 2.0 + 0.05);
    let segment_half = (total_height * 0.5 - radius).max(0.0);
    let mut best = f32::INFINITY;

    let cylinder_a = dir.x * dir.x + dir.z * dir.z;
    if cylinder_a > RAY_EPSILON {
        let cylinder_b = 2.0 * (origin.x * dir.x + origin.z * dir.z);
        let cylinder_c = origin.x * origin.x + origin.z * origin.z - radius * radius;
        let discriminant = cylinder_b * cylinder_b - 4.0 * cylinder_a * cylinder_c;
        if discriminant >= 0.0 {
            let root = discriminant.sqrt();
            for distance in [
                (-cylinder_b - root) / (2.0 * cylinder_a),
                (-cylinder_b + root) / (2.0 * cylinder_a),
            ] {
                let y = origin.y + dir.y * distance;
                if distance > RAY_EPSILON && y >= -segment_half && y <= segment_half {
                    best = best.min(distance);
                }
            }
        }
    }

    for center_y in [-segment_half, segment_half] {
        if let Some(distance) = sphere_intersection(origin, dir, Vec3::new(0.0, center_y, 0.0), radius) {
            best = best.min(distance);
        }
    }
    best.is_finite().then_some(best)
}

pub(crate) fn sphere_intersection(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let offset = origin - center;
    let a = dir.dot(dir);
    if a <= RAY_EPSILON {
        return None;
    }
    let half_b = offset.dot(dir);
    let c = offset.length_squared() - radius * radius;
    let discriminant = half_b * half_b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = (-half_b - root) / a;
    if near > RAY_EPSILON {
        return Some(near);
    }
    let far = (-half_b + root) / a;
    (far > RAY_EPSILON).then_some(far)
}

fn triangle_intersection(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let edge_ab = b - a;
    let edge_ac = c - a;
    let p = dir.cross(edge_ac);
    let determinant = edge_ab.dot(p);
    if determinant.abs() <= RAY_EPSILON {
        return None;
    }
    let inverse = 1.0 / determinant;
    let from_a = origin - a;
    let u = from_a.dot(p) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = from_a.cross(edge_ab);
    let v = dir.dot(q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge_ac.dot(q) * inverse;
    (distance > RAY_EPSILON).then_some(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thin_ground_does_not_mask_cube_above_it() {
        let origin = Vec3::new(0.0, 3.0, 8.0);
        let direction = (Vec3::ZERO - origin).normalize();
        let ground = GlobalTransform {
            translation: Vec3::new(0.0, -1.0, 0.0),
            ..GlobalTransform::default()
        };
        let cube = GlobalTransform::default();
        let ground_shape = Shape {
            primitive: PrimitiveShape::Plane,
            size: Vec3::new(20.0, 0.0, 20.0),
        };
        let cube_shape = Shape {
            primitive: PrimitiveShape::Cube,
            size: Vec3::ONE,
        };

        let cube_distance = primitive_intersection(origin, direction, &cube, &cube_shape).unwrap();
        let ground_distance = primitive_intersection(origin, direction, &ground, &ground_shape).unwrap();
        assert!(cube_distance < ground_distance);
    }

    #[test]
    fn plane_only_hits_inside_its_rectangle() {
        let transform = GlobalTransform::default();
        let shape = Shape {
            primitive: PrimitiveShape::Plane,
            size: Vec3::new(2.0, 0.0, 2.0),
        };
        assert!(primitive_intersection(
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::NEG_Y,
            &transform,
            &shape,
        ).is_some());
        assert!(primitive_intersection(
            Vec3::new(3.0, 2.0, 0.0),
            Vec3::NEG_Y,
            &transform,
            &shape,
        ).is_none());
    }

    #[test]
    fn mesh_triangle_returns_world_ray_distance() {
        let mesh = MeshAsset {
            vertices: vec![
                vetrace_render::MeshVertex { position: Vec3::new(-1.0, -1.0, 0.0), ..Default::default() },
                vetrace_render::MeshVertex { position: Vec3::new(1.0, -1.0, 0.0), ..Default::default() },
                vetrace_render::MeshVertex { position: Vec3::new(0.0, 1.0, 0.0), ..Default::default() },
            ],
            indices: vec![0, 1, 2],
            ..Default::default()
        };
        let transform = GlobalTransform {
            translation: Vec3::new(0.0, 0.0, -5.0),
            scale: Vec3::new(2.0, 3.0, 1.0),
            ..GlobalTransform::default()
        };
        let distance = mesh_intersection(Vec3::ZERO, Vec3::NEG_Z, &transform, &mesh).unwrap();
        assert!((distance - 5.0).abs() < 1.0e-4);
    }
}
