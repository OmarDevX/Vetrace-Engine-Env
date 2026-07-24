export const EXAMPLES = [
  {
    slug: "rotating-cube",
    title: "Rotating Cube",
    category: "Basics",
    description: "A minimal Vetrace scene with one animated actor, a material, a camera, and directional lighting.",
    tags: ["Actor", "Transform", "Animation"],
    complexity: "Beginner",
    source: `use glam::{Quat, Vec3};
use vetrace_core::{Engine, Stage};
use vetrace_render::{Material, PrimitiveShape, Renderable, Shape};

let cube = engine
    .spawn_actor("Rotating Cube")
    .with(Shape {
        primitive: PrimitiveShape::Cube,
        size: Vec3::splat(2.15),
    })
    .with(Material {
        base_color: Vec3::new(0.16, 0.68, 1.0),
        roughness: 0.24,
        metallic: 0.12,
        ..Material::default()
    })
    .with(Renderable {
        mesh: None,
        material: None,
        visible: true,
    })
    .build();

cube.set_rotation(
    &mut engine,
    Quat::from_rotation_y(time * 0.8)
        * Quat::from_rotation_x(time * 0.32),
)?;

engine.run_stage(Stage::RenderExtract, dt);`
  },
  {
    slug: "shapes",
    title: "3D Shapes",
    category: "3D Rendering",
    description: "Built-in cube, sphere, capsule, and plane geometry rendered from normal Vetrace components.",
    tags: ["Primitives", "Materials", "Lighting"],
    complexity: "Beginner",
    source: `for (name, primitive, position, color) in shapes {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .with(Shape {
            primitive,
            size: Vec3::splat(1.7),
        })
        .with(Material {
            base_color: color,
            roughness: 0.35,
            metallic: 0.08,
            ..Material::default()
        })
        .with(Renderable {
            mesh: None,
            material: None,
            visible: true,
        })
        .build();
}`
  },
  {
    slug: "materials",
    title: "Material Grid",
    category: "3D Rendering",
    description: "Compare roughness and metallic response across a row of live browser-rendered spheres.",
    tags: ["PBR", "Roughness", "Metallic"],
    complexity: "Beginner",
    source: `for index in 0..7 {
    let metallic = index as f32 / 6.0;
    let roughness = 0.08 + (1.0 - metallic) * 0.78;

    let x = (index as f32 - 3.0) * 1.65;
    engine
        .spawn_actor(format!("Material {index}"))
        .with(Transform {
            translation: Vec3::new(x, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .with(Shape {
            primitive: PrimitiveShape::Sphere,
            size: Vec3::splat(1.45),
        })
        .with(Material {
            base_color: Vec3::new(
                0.12 + metallic * 0.7,
                0.42,
                0.96 - metallic * 0.45,
            ),
            roughness,
            metallic,
            ..Material::default()
        })
        .with(Renderable {
            mesh: None,
            material: None,
            visible: true,
        })
        .build();
}`
  },
  {
    slug: "lighting",
    title: "Dynamic Lighting",
    category: "3D Rendering",
    description: "Directional and colored point lights illuminate a small scene using the browser WGPU backend.",
    tags: ["PointLight", "DirectionalLight", "Emissive"],
    complexity: "Intermediate",
    source: `let light = engine
    .spawn_actor("Blue Light")
    .with(Transform {
        translation: Vec3::new(0.0, 2.0, 2.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    })
    .with(Shape {
        primitive: PrimitiveShape::Sphere,
        size: Vec3::splat(0.34),
    })
    .with(Material {
        base_color: Vec3::new(0.08, 0.45, 1.0),
        emissive: Vec3::new(0.2, 1.2, 2.6),
        ..Material::default()
    })
    .with(Renderable {
        mesh: None,
        material: None,
        visible: true,
    })
    .with(PointLight {
        color: Vec3::new(0.08, 0.45, 1.0),
        intensity: 8.0,
        range: Some(7.5),
        shadow_mode: ShadowMode::None,
    })
    .build();`
  },
  {
    slug: "many-cubes",
    title: "Many Cubes",
    category: "Stress Tests",
    description: "A dense animated field of actors exercises extraction, transforms, CPU geometry, and browser uploads.",
    tags: ["Stress Test", "ECS", "Rendering"],
    complexity: "Intermediate",
    source: `for z in -6..=6 {
    for x in -6..=6 {
        let position = Vec3::new(x as f32 * 1.05, 0.5, z as f32 * 1.05);
        engine
            .spawn_actor(format!("Cube {x} {z}"))
            .with(Transform {
                translation: position,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .with(Shape {
                primitive: PrimitiveShape::Cube,
                size: Vec3::splat(0.74),
            })
            .with(Material::default())
            .with(Renderable {
                mesh: None,
                material: None,
                visible: true,
            })
            .build();
    }
}`
  },
  {
    slug: "hierarchy",
    title: "Transform Hierarchy",
    category: "ECS & Transforms",
    description: "Parent and child actors demonstrate local transforms and automatic global-transform propagation.",
    tags: ["Parenting", "GlobalTransform", "Actor"],
    complexity: "Intermediate",
    source: `let parent = engine
    .spawn_actor("Hierarchy Root")
    .with(Transform {
        translation: Vec3::Y,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    })
    .with(Shape {
        primitive: PrimitiveShape::Cube,
        size: Vec3::splat(1.6),
    })
    .with(Renderable {
        mesh: None,
        material: None,
        visible: true,
    })
    .build();

for index in 0..5 {
    engine
        .spawn_actor(format!("Child {index}"))
        .with(Transform {
            translation: child_offset(index),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .with(Shape {
            primitive: PrimitiveShape::Sphere,
            size: Vec3::splat(0.9),
        })
        .with(Renderable {
            mesh: None,
            material: None,
            visible: true,
        })
        .child_of(parent)?
        .build();
}`
  },
  {
    slug: "camera-controls",
    title: "Orbit Camera",
    category: "Input",
    description: "Drag, scroll, or use WASD/arrow keys to control a browser camera through Vetrace InputState.",
    tags: ["InputState", "Camera", "Pointer"],
    complexity: "Beginner",
    source: `let input = engine
    .get_resource::<InputState>()
    .expect("InputState is installed by the runtime");
let (dx, dy) = input.mouse_delta();

if input.is_mouse_button_down("Left") {
    orbit.yaw -= dx * 0.006;
    orbit.pitch = (orbit.pitch + dy * 0.006)
        .clamp(-1.15, 1.15);
}

orbit.distance = (orbit.distance
    + input.mouse_wheel_delta().1 * 0.008)
    .clamp(4.0, 28.0);

let horizontal = orbit.distance * orbit.pitch.cos();
camera.position = target + Vec3::new(
    orbit.yaw.sin() * horizontal,
    orbit.distance * orbit.pitch.sin(),
    orbit.yaw.cos() * horizontal,
);
camera.target = target;`
  }
];

export const CATEGORIES = ["All", ...new Set(EXAMPLES.map((example) => example.category))];

export function exampleBySlug(slug) {
  return EXAMPLES.find((example) => example.slug === slug) ?? EXAMPLES[0];
}
