//! Clean primitive construction helpers.
//!
//! This crate is intentionally small and data-oriented. It owns no editor UI,
//! no game policy, and no renderer internals. Games/tools pass options in and
//! receive a normal ECS entity with `Transform`, `Shape`, `Material`, and
//! optional physics components.

use glam::{Quat, Vec3};
#[cfg(feature = "render_2d")]
use glam::{Vec2, Vec4};
use serde::{Deserialize, Serialize};
use vetrace_core::{Actor, Engine, Entity, Transform};
use vetrace_render::{Material, PrimitiveShape, Renderable, Shape};
#[cfg(feature = "render_2d")]
use vetrace_render::{CanvasItem2D, Sprite2D, TextureFilter2D, TextureHandle};

#[cfg(feature = "physics")]
use vetrace_physics::{Collider, ColliderShape, StaticBody};

/// User-facing primitive kind used by tools and prefab files.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveKind {
    #[default]
    Cube,
    Sphere,
    Capsule,
    Plane,
    Quad,
}

impl From<PrimitiveKind> for PrimitiveShape {
    fn from(value: PrimitiveKind) -> Self {
        match value {
            PrimitiveKind::Cube => PrimitiveShape::Cube,
            PrimitiveKind::Sphere => PrimitiveShape::Sphere,
            PrimitiveKind::Capsule => PrimitiveShape::Capsule,
            PrimitiveKind::Plane => PrimitiveShape::Plane,
            PrimitiveKind::Quad => PrimitiveShape::Quad,
        }
    }
}

impl From<PrimitiveShape> for PrimitiveKind {
    fn from(value: PrimitiveShape) -> Self {
        match value {
            PrimitiveShape::Cube => PrimitiveKind::Cube,
            PrimitiveShape::Sphere => PrimitiveKind::Sphere,
            PrimitiveShape::Capsule => PrimitiveKind::Capsule,
            PrimitiveShape::Plane => PrimitiveKind::Plane,
            PrimitiveShape::Quad => PrimitiveKind::Quad,
        }
    }
}

/// Optional static collider policy for primitive spawns.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrimitiveColliderOptions {
    pub enabled: bool,
    pub half_extents: Vec3,
}

impl PrimitiveColliderOptions {
    pub fn disabled() -> Self {
        Self { enabled: false, half_extents: Vec3::splat(0.5) }
    }

    pub fn static_solid(size: Vec3) -> Self {
        Self {
            enabled: true,
            half_extents: collider_half_extents(size),
        }
    }
}

impl Default for PrimitiveColliderOptions {
    fn default() -> Self { Self::disabled() }
}

/// Options used by all primitive spawners.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrimitiveSpawnOptions {
    pub name: String,
    pub primitive: PrimitiveKind,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub size: Vec3,
    pub color: Vec3,
    pub visible: bool,
    pub tags: Vec<String>,
    pub collider: PrimitiveColliderOptions,
}

impl Default for PrimitiveSpawnOptions {
    fn default() -> Self {
        Self {
            name: "Primitive".to_string(),
            primitive: PrimitiveKind::Cube,
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            size: Vec3::ONE,
            color: Vec3::new(0.45, 0.55, 0.70),
            visible: true,
            tags: Vec::new(),
            collider: PrimitiveColliderOptions::disabled(),
        }
    }
}

/// Spawns a normal renderable primitive. Physics support is compiled in only
/// when this crate's `physics` feature is enabled.
pub fn spawn_primitive_actor(engine: &mut Engine, options: PrimitiveSpawnOptions) -> Actor {
    let actor = engine
        .spawn_actor(options.name.clone())
        .with(Transform {
            translation: options.translation,
            rotation: options.rotation,
            scale: options.scale,
        })
        .with(Shape {
            primitive: options.primitive.into(),
            size: options.size.max(Vec3::splat(0.001)),
        })
        .with(Material {
            base_color: options.color.clamp(Vec3::ZERO, Vec3::ONE),
            roughness: 0.75,
            ..Material::default()
        })
        .with(Renderable { visible: options.visible, ..Renderable::default() })
        .source("vetrace_primitives")
        .build();

    for tag in options.tags { let _ = actor.add_tag(engine, tag); }

    #[cfg(feature = "physics")]
    if options.collider.enabled {
        let _ = actor.insert(engine, StaticBody::default());
        let _ = actor.insert(engine, Collider {
            shape: collider_shape(options.primitive),
            half_extents: options.collider.half_extents.max(Vec3::splat(0.001)),
            ..Collider::default()
        });
    }

    actor
}


#[cfg(feature = "render_2d")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sprite2DSpawnOptions {
    pub name: String,
    pub texture: Option<TextureHandle>,
    pub texture_path: Option<String>,
    pub position: Vec2,
    pub rotation_radians: f32,
    pub scale: Vec2,
    pub size: Vec2,
    pub pivot: Vec2,
    pub tint: Vec4,
    pub filter: TextureFilter2D,
    pub pixel_snap: bool,
    pub canvas_layer: i32,
    pub z_index: i32,
    pub visible: bool,
}

#[cfg(feature = "render_2d")]
impl Default for Sprite2DSpawnOptions {
    fn default() -> Self {
        Self {
            name: "Sprite 2D".to_string(),
            texture: None,
            texture_path: None,
            position: Vec2::ZERO,
            rotation_radians: 0.0,
            scale: Vec2::ONE,
            size: Vec2::ONE,
            pivot: Vec2::splat(0.5),
            tint: Vec4::ONE,
            filter: TextureFilter2D::Linear,
            pixel_snap: false,
            canvas_layer: 0,
            z_index: 0,
            visible: true,
        }
    }
}

#[cfg(feature = "render_2d")]
pub fn spawn_sprite_2d_actor(engine: &mut Engine, options: Sprite2DSpawnOptions) -> Actor {
    engine
        .spawn_actor(options.name)
        .with(Transform {
            translation: Vec3::new(options.position.x, options.position.y, 0.0),
            rotation: Quat::from_rotation_z(options.rotation_radians),
            scale: Vec3::new(options.scale.x, options.scale.y, 1.0),
        })
        .with(Sprite2D {
            texture: options.texture,
            texture_path: options.texture_path,
            size: options.size.abs().max(Vec2::splat(0.0001)),
            pivot: options.pivot,
            tint: options.tint,
            filter: options.filter,
            pixel_snap: options.pixel_snap,
            ..Sprite2D::default()
        })
        .with(CanvasItem2D {
            visible: options.visible,
            canvas_layer: options.canvas_layer,
            z_index: options.z_index,
            ..CanvasItem2D::default()
        })
        .source("vetrace_primitives")
        .build()
}

/// Compatibility wrapper. New gameplay and tools should keep the returned Actor.
#[deprecated(note = "use spawn_primitive_actor")]
pub fn spawn_primitive(engine: &mut Engine, options: PrimitiveSpawnOptions) -> Entity {
    spawn_primitive_actor(engine, options).entity()
}

pub fn cube(name: impl Into<String>, translation: Vec3) -> PrimitiveSpawnOptions {
    PrimitiveSpawnOptions {
        name: name.into(),
        primitive: PrimitiveKind::Cube,
        translation,
        collider: PrimitiveColliderOptions::static_solid(Vec3::ONE),
        ..PrimitiveSpawnOptions::default()
    }
}

pub fn sphere(name: impl Into<String>, translation: Vec3) -> PrimitiveSpawnOptions {
    PrimitiveSpawnOptions {
        name: name.into(),
        primitive: PrimitiveKind::Sphere,
        translation,
        collider: PrimitiveColliderOptions::static_solid(Vec3::ONE),
        ..PrimitiveSpawnOptions::default()
    }
}

pub fn capsule(name: impl Into<String>, translation: Vec3) -> PrimitiveSpawnOptions {
    PrimitiveSpawnOptions {
        name: name.into(),
        primitive: PrimitiveKind::Capsule,
        translation,
        size: Vec3::new(1.0, 2.0, 1.0),
        collider: PrimitiveColliderOptions::static_solid(Vec3::new(1.0, 2.0, 1.0)),
        ..PrimitiveSpawnOptions::default()
    }
}

pub fn plane(name: impl Into<String>, translation: Vec3, size: Vec3) -> PrimitiveSpawnOptions {
    PrimitiveSpawnOptions {
        name: name.into(),
        primitive: PrimitiveKind::Plane,
        translation,
        size,
        collider: PrimitiveColliderOptions::disabled(),
        ..PrimitiveSpawnOptions::default()
    }
}

pub fn collider_half_extents(size: Vec3) -> Vec3 {
    let s = size.abs().max(Vec3::splat(0.001));
    Vec3::new(s.x * 0.5, s.y * 0.5, s.z * 0.5)
}

#[cfg(feature = "physics")]
fn collider_shape(kind: PrimitiveKind) -> ColliderShape {
    match kind {
        PrimitiveKind::Sphere => ColliderShape::Sphere,
        PrimitiveKind::Capsule => ColliderShape::Capsule,
        PrimitiveKind::Cube | PrimitiveKind::Plane | PrimitiveKind::Quad => ColliderShape::Cube,
    }
}

/// Tags used by official tools so runtime importers can distinguish authored
/// map geometry from temporary helper/editor entities.
pub mod tags {
    pub const PREFAB_OBJECT: &str = "prefab_object";
    pub const MAP_BUILDER_AUTHORED: &str = "map_builder_authored";
    pub const MAP_BUILDER_HELPER: &str = "map_builder_helper";
}

/// Human-readable primitive names used by UI/tools.
pub fn primitive_display_name(kind: PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::Cube => "Cube",
        PrimitiveKind::Sphere => "Sphere",
        PrimitiveKind::Capsule => "Capsule",
        PrimitiveKind::Plane => "Plane",
        PrimitiveKind::Quad => "Quad",
    }
}
