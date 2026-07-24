use super::*;

pub(crate) fn build_object_draw_commands(
    object: &RenderObject,
    frame: &RenderFrame,
    width: f32,
    height: f32,
    faces: &mut Vec<DrawFace>,
    wires: &mut Vec<DrawWire>,
) {
    let shape = object.shape.as_ref().cloned().unwrap_or_default_shape();
    let Some(world_vertices) = shape_world_vertices(&shape, object) else { return; };

    // Cheap distance/frustum culling before triangle fill. This matters a lot for
    // the software fallback when the game spawns many ground tiles.
    let center = object.transform.translation;
    let distance_to_camera = center.distance(frame.camera.position);
    if distance_to_camera > frame.camera.far { return; }
    let view_dir = (frame.camera.target - frame.camera.position).normalize_or_zero();
    if view_dir.length_squared() > 0.0 && (center - frame.camera.position).dot(view_dir) < -2.0 { return; }

    let mut projected = [Vec2::ZERO; 8];
    for (index, point) in world_vertices.iter().enumerate() {
        let Some(screen) = project_to_screen(*point, &frame.camera, width, height) else { return; };
        projected[index] = screen;
    }

    if projected.iter().all(|p| p.x < -64.0 || p.x > width + 64.0 || p.y < -64.0 || p.y > height + 64.0) {
        return;
    }

    let depth = world_vertices
        .iter()
        .map(|p| p.distance(frame.camera.position))
        .sum::<f32>()
        / world_vertices.len() as f32;

    let material = effective_material(object, frame);
    let wire_only = material.alpha <= 0.15 || object.name.as_deref().map(is_outline_name).unwrap_or(false);
    let base_wire_color = opaque(color_from_material(&material, Vec3::Y, frame, 1.0));

    if wire_only {
        wires.push(DrawWire { points: projected, depth, color: base_wire_color });
        return;
    }

    for face in cube_faces() {
        let normal = face_normal(&world_vertices, face.indices);
        let color = color_from_material(&material, normal, frame, face.brightness);
        let points = [
            projected[face.indices[0]],
            projected[face.indices[1]],
            projected[face.indices[2]],
            projected[face.indices[3]],
        ];
        let face_depth = face.indices.iter().map(|&i| world_vertices[i].distance(frame.camera.position)).sum::<f32>() / 4.0;
        faces.push(DrawFace { points, depth: face_depth, color });
    }

    // Subtle wire overlay makes the fallback read as 3D even without a real z-buffer.
    wires.push(DrawWire { points: projected, depth: depth - 0.001, color: darken(base_wire_color, 0.25) });

    // Renderer-side outline fallback. A GPU backend can replace this with an
    // actual silhouette pass; the software backend draws an expanded wire hull.
    if let Some(outline) = &object.outline {
        if outline.enabled {
            if let Some(outline_vertices) = shape_world_vertices_with_extra(&shape, object, outline.thickness.max(0.0)) {
                let mut outline_projected = [Vec2::ZERO; 8];
                let mut ok = true;
                for (index, point) in outline_vertices.iter().enumerate() {
                    if let Some(screen) = project_to_screen(*point, &frame.camera, width, height) {
                        outline_projected[index] = screen;
                    } else {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    let c = outline.color.clamp(Vec3::ZERO, Vec3::ONE);
                    wires.push(DrawWire {
                        points: outline_projected,
                        depth: depth - 0.002,
                        color: Color::RGBA((c.x * 255.0) as u8, (c.y * 255.0) as u8, (c.z * 255.0) as u8, 255),
                    });
                }
            }
        }
    }
}

pub(crate) fn effective_material(object: &RenderObject, frame: &RenderFrame) -> Material {
    let mut material = object.material.clone();
    if let Some(shader) = &object.custom_shader {
        // Software fallback for CustomShaderMaterial. GPU backends should compile
        // shader.wgsl_source/asset_path and bind params; this keeps the same game
        // component working in the fallback path.
        let time = frame.settings.time_seconds;
        let seed = shader.params.first().copied().unwrap_or(0.0);
        let health01 = shader.params.get(1).copied().unwrap_or(1.0).clamp(0.0, 1.0);
        let t = 0.5 + 0.5 * (object.transform.translation.y * 2.5 + time * 3.0 + seed).sin();
        let color = shader.fallback_color_a.lerp(shader.fallback_color_b, t).clamp(Vec3::splat(0.05), Vec3::ONE) * (0.35 + health01 * 0.65);
        material.base_color = color;
        material.emissive = color * 0.08;
    }
    material
}


trait DefaultShapeExt {
    fn unwrap_or_default_shape(self) -> Shape;
}

impl DefaultShapeExt for Option<Shape> {
    fn unwrap_or_default_shape(self) -> Shape {
        self.unwrap_or(Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE })
    }
}

fn shape_world_vertices(shape: &Shape, object: &RenderObject) -> Option<[Vec3; 8]> {
    shape_world_vertices_with_extra(shape, object, 0.0)
}

fn shape_world_vertices_with_extra(shape: &Shape, object: &RenderObject, extra: f32) -> Option<[Vec3; 8]> {
    let size = match shape.primitive {
        PrimitiveShape::Cube => shape.size.max(Vec3::splat(0.01)),
        PrimitiveShape::Plane => Vec3::new(shape.size.x.max(0.01), 0.02, shape.size.z.max(shape.size.y).max(0.01)),
        PrimitiveShape::Quad => Vec3::new(shape.size.x.max(0.01), shape.size.y.max(0.01), 0.02),
        // Temporary visual approximations until mesh/sphere/capsule raster paths are added.
        PrimitiveShape::Sphere => Vec3::splat(shape.size.max_element().max(0.05)),
        PrimitiveShape::Capsule => Vec3::new(shape.size.x.max(0.05), shape.size.y.max(0.1), shape.size.z.max(0.05)),
    };

    let half = size * 0.5 + Vec3::splat(extra);
    let local = [
        Vec3::new(-half.x, -half.y, -half.z),
        Vec3::new( half.x, -half.y, -half.z),
        Vec3::new( half.x,  half.y, -half.z),
        Vec3::new(-half.x,  half.y, -half.z),
        Vec3::new(-half.x, -half.y,  half.z),
        Vec3::new( half.x, -half.y,  half.z),
        Vec3::new( half.x,  half.y,  half.z),
        Vec3::new(-half.x,  half.y,  half.z),
    ];

    let transform = &object.transform;
    Some(local.map(|p| transform.translation + transform.rotation * (p * transform.scale)))
}

#[derive(Clone, Copy)]
struct FaceDef {
    indices: [usize; 4],
    brightness: f32,
}

fn cube_faces() -> [FaceDef; 6] {
    [
        FaceDef { indices: [0, 1, 2, 3], brightness: 0.95 }, // back
        FaceDef { indices: [5, 4, 7, 6], brightness: 1.00 }, // front
        FaceDef { indices: [4, 0, 3, 7], brightness: 0.82 }, // left
        FaceDef { indices: [1, 5, 6, 2], brightness: 0.88 }, // right
        FaceDef { indices: [3, 2, 6, 7], brightness: 1.15 }, // top
        FaceDef { indices: [4, 5, 1, 0], brightness: 0.65 }, // bottom
    ]
}

fn face_normal(vertices: &[Vec3; 8], indices: [usize; 4]) -> Vec3 {
    let a = vertices[indices[0]];
    let b = vertices[indices[1]];
    let c = vertices[indices[2]];
    (b - a).cross(c - a).normalize_or_zero()
}

pub(crate) fn color_from_material(material: &Material, normal: Vec3, frame: &RenderFrame, face_brightness: f32) -> Color {
    let mut light = 0.30 * face_brightness;
    for directional in &frame.directional_lights {
        let light_dir = (-directional.direction).normalize_or_zero();
        let ndotl = normal.normalize_or_zero().dot(light_dir).max(0.0);
        light += ndotl * directional.intensity.max(0.0) * 0.38;
    }
    if frame.directional_lights.is_empty() {
        light += 0.45;
    }
    let fog_tint = frame.fog.as_ref().filter(|fog| fog.enabled).map(|fog| fog.color * fog.density.min(1.0) * 8.0).unwrap_or(Vec3::ZERO);
    let color = (material.base_color * light + material.emissive + fog_tint).clamp(Vec3::ZERO, Vec3::ONE);
    Color::RGBA(
        (color.x * 255.0) as u8,
        (color.y * 255.0) as u8,
        (color.z * 255.0) as u8,
        (material.alpha.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

pub(crate) fn is_outline_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("outline")
}
