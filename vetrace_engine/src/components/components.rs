use crate::ecs::Component;
use crate::gpu::MeshHandle;
use crate::inspector::export::{ExportKind, ExportedField};
use crate::inspector::Inspectable;
use crate::materials::PbrMaterial;
use crate::net::sync::NetSyncComponent;
use glam::{Vec2, Vec3};
use rapier3d::na::{
    Isometry3 as Isometry, Quaternion, Translation3 as Translation, UnitQuaternion,
};
use rapier3d::prelude::{RigidBodyHandle, SharedShape};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug)]
pub struct Rotate {
    pub speed: f32,
}
impl Component for Rotate {}
impl Default for Rotate {
    fn default() -> Self {
        Self { speed: 11.0 }
    }
}
impl Inspectable for Rotate {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "speed",
            kind: ExportKind::Slider {
                min: 0.0,
                max: 50.0,
            },
            value: &mut self.speed as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<f32>(),
        }]
    }
}
#[derive(Default, Debug)]
pub struct Player;

// ✅ This is the missing part!
impl Component for Player {}
impl Inspectable for Player {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug)]
pub struct ObjectRef {
    pub id: u32,
}
impl Component for ObjectRef {}

#[derive(Debug)]
pub struct Material {
    pub is_glass: bool,
    pub specular_f0: Vec3,
    pub ior: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            is_glass: false,
            specular_f0: Vec3::ZERO,
            ior: 1.5,
        }
    }
}

impl Component for Material {}
impl Inspectable for Material {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "is_glass",
                kind: ExportKind::Checkbox,
                value: &mut self.is_glass as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "f0_r",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.specular_f0.x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "f0_g",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.specular_f0.y as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "f0_b",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.specular_f0.z as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "ior",
                kind: ExportKind::Slider { min: 1.0, max: 3.0 },
                value: &mut self.ior as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl Component for MeshHandle {}
impl Inspectable for MeshHandle {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

impl Component for PbrMaterial {}
impl Inspectable for PbrMaterial {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColliderShape {
    Sphere = 0,
    Cube = 1,
    Capsule = 2,
}

impl Default for ColliderShape {
    fn default() -> Self {
        ColliderShape::Sphere
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    /// Explicit collider shape
    pub shape: ColliderShape,
    /// Local offset of the collider relative to entity transform
    pub position: [f32; 3],
    /// Local orientation as quaternion `[x, y, z, w]`
    pub rotation: [f32; 4],
    /// Size parameters used by explicit shapes
    pub size: [f32; 3],
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            shape: ColliderShape::Sphere,
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            size: [1.0, 1.0, 1.0],
        }
    }
}

impl Component for Collider {}
impl Inspectable for Collider {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "shape",
                kind: ExportKind::Slider { min: 0.0, max: 2.0 },
                value: &mut self.shape as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<ColliderShape>(),
            },
            ExportedField {
                name: "pos_x",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.position[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "pos_y",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.position[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "pos_z",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.position[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "rot_x",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.rotation[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "rot_y",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.rotation[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "rot_z",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.rotation[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "rot_w",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.rotation[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_z",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.size[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl Collider {
    /// Compute the collider's isometry by combining parent transform with local offset
    pub fn iso(&self, parent: &Transform) -> Isometry<f32> {
        let parent_iso = parent.iso();
        let q = UnitQuaternion::from_quaternion(Quaternion::new(
            self.rotation[3],
            self.rotation[0],
            self.rotation[1],
            self.rotation[2],
        ));
        parent_iso
            * Isometry::from_parts(
                Translation::new(self.position[0], self.position[1], self.position[2]),
                q,
            )
    }

    /// Build a `SharedShape` representing this collider
    pub fn shape(&self) -> SharedShape {
        match self.shape {
            ColliderShape::Sphere => SharedShape::ball(self.size[0] * 0.5),
            ColliderShape::Cube => {
                SharedShape::cuboid(self.size[0] * 0.5, self.size[1] * 0.5, self.size[2] * 0.5)
            }
            ColliderShape::Capsule => {
                let radius = self.size[0] * 0.5;
                let half_height = (self.size[1] * 0.5 - radius).max(0.0);
                SharedShape::capsule_y(half_height, radius)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct StaticBody {
    pub handle: Option<rapier3d::prelude::RigidBodyHandle>,
}
impl Component for StaticBody {}
impl Inspectable for StaticBody {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug, Default)]
pub struct KinematicBody {
    pub handle: Option<rapier3d::prelude::RigidBodyHandle>,
}
impl Component for KinematicBody {}
impl Inspectable for KinematicBody {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug, Default)]
pub struct RevoluteJoint {
    pub other: u32,
    pub axis: [f32; 3],
    pub contacts_enabled: bool,
    pub handle: Option<rapier3d::prelude::ImpulseJointHandle>,
}
impl Component for RevoluteJoint {}
impl Inspectable for RevoluteJoint {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "other",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 1000.0,
                },
                value: &mut self.other as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<u32>(),
            },
            ExportedField {
                name: "axis_x",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.axis[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "axis_y",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.axis[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "axis_z",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.axis[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "contacts_enabled",
                kind: ExportKind::Checkbox,
                value: &mut self.contacts_enabled as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

#[derive(Debug, Default)]
pub struct BallJoint {
    pub other: u32,
    pub contacts_enabled: bool,
    pub handle: Option<rapier3d::prelude::ImpulseJointHandle>,
}
impl Component for BallJoint {}
impl Inspectable for BallJoint {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "other",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 1000.0,
                },
                value: &mut self.other as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<u32>(),
            },
            ExportedField {
                name: "contacts_enabled",
                kind: ExportKind::Checkbox,
                value: &mut self.contacts_enabled as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct RigidBody3D {
    pub gravity_enabled: bool,
    pub gravity_force: [f32; 3],
    pub mass: f32,
    pub friction: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub bounciness: f32,
    pub handle: Option<rapier3d::prelude::RigidBodyHandle>,
}

impl Component for RigidBody3D {}

impl Default for RigidBody3D {
    fn default() -> Self {
        Self {
            gravity_enabled: true,
            gravity_force: [0.0, -9.81, 0.0],
            mass: 1.0,
            friction: 0.5,
            linear_damping: 0.1,
            angular_damping: 0.1,
            bounciness: 0.0,
            handle: None,
        }
    }
}

impl Inspectable for RigidBody3D {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "gravity_enabled",
                kind: ExportKind::Checkbox,
                value: &mut self.gravity_enabled as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "gravity_force_x",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.gravity_force[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "gravity_force_y",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.gravity_force[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "gravity_force_z",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.gravity_force[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "mass",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 100.0,
                },
                value: &mut self.mass as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "friction",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.friction as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "linear_damping",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.linear_damping as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_damping",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.angular_damping as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "bounciness",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.bounciness as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Default, Debug)]
pub struct Renderable {
    pub color: [f32; 3],
    pub roughness: f32,
    pub emission: f32,
    pub is_mesh: bool,
    pub triangle_start_idx: u32,
    pub triangle_count: u32,
}
impl Component for Renderable {}
impl Inspectable for Renderable {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "color_r",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_g",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_b",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "roughness",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.roughness as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "emission",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.emission as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "is_mesh",
                kind: ExportKind::Checkbox,
                value: &mut self.is_mesh as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "triangle_start_idx",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10000.0,
                },
                value: &mut self.triangle_start_idx as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<u32>(),
            },
            ExportedField {
                name: "triangle_count",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10000.0,
                },
                value: &mut self.triangle_count as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<u32>(),
            },
        ]
    }
}

#[derive(Default, Debug)]
pub struct ObjMesh {
    pub path: String,
    pub loaded: bool,
}
impl Component for ObjMesh {}
impl Inspectable for ObjMesh {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "path",
            kind: ExportKind::Text,
            value: &mut self.path as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<String>(),
        }]
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct AngularVelocity {
    pub angular_velocity: [f32; 3],
    pub angular_acceleration: [f32; 3],
}
impl Component for AngularVelocity {}
impl Inspectable for AngularVelocity {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "angular_velocity_x",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.angular_velocity[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_velocity_y",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.angular_velocity[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_velocity_z",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.angular_velocity[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_acceleration_x",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.angular_acceleration[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_acceleration_y",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.angular_acceleration[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_acceleration_z",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.angular_acceleration[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Velocity {
    pub velocity: [f32; 3],
    pub acceleration: [f32; 3],
}
impl Component for Velocity {}
impl Inspectable for Velocity {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "velocity_x",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.velocity[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "velocity_y",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.velocity[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "velocity_z",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.velocity[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "acceleration_x",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.acceleration[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "acceleration_y",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.acceleration[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "acceleration_z",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.acceleration[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl NetSyncComponent for Velocity {
    fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn deserialize(&mut self, data: &[u8]) {
        if let Ok(v) = bincode::deserialize::<Self>(data) {
            *self = v;
        }
    }

    fn has_changed(&self) -> bool {
        true
    }

    fn component_name() -> &'static str {
        "Velocity"
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub position: [f32; 3],
    pub orientation: [f32; 4], // [x, y, z, w]
    pub size: [f32; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            orientation: [0.0, 0.0, 0.0, 1.0],
            size: [1.0, 1.0, 1.0],
        }
    }
}
impl Component for Transform {}
impl Inspectable for Transform {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "position_x",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.position[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "position_y",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.position[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "position_z",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.position[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "orientation_w",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.orientation[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "orientation_x",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.orientation[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "orientation_y",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.orientation[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "orientation_z",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.orientation[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 100.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 100.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_z",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 100.0,
                },
                value: &mut self.size[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl NetSyncComponent for Transform {
    fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn deserialize(&mut self, data: &[u8]) {
        if let Ok(t) = bincode::deserialize::<Self>(data) {
            *self = t;
        }
    }

    fn has_changed(&self) -> bool {
        true
    }

    fn component_name() -> &'static str {
        "Transform"
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalTransform {
    pub position: [f32; 3],
    pub orientation: [f32; 4],
    pub size: [f32; 3],
}

impl Component for GlobalTransform {}

impl Transform {
    pub fn iso(&self) -> Isometry<f32> {
        let q = UnitQuaternion::from_quaternion(Quaternion::new(
            self.orientation[3],
            self.orientation[0],
            self.orientation[1],
            self.orientation[2],
        ));
        Isometry::from_parts(
            Translation::new(self.position[0], self.position[1], self.position[2]),
            q,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Parent {
    pub entity: crate::ecs::Entity,
}

impl Component for Parent {}
impl Default for Parent {
    fn default() -> Self {
        Self {
            entity: crate::ecs::Entity(0),
        }
    }
}
impl Inspectable for Parent {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "parent_id",
            kind: ExportKind::Slider {
                min: 0.0,
                max: 1000.0,
            },
            value: &mut self.entity.0 as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<u32>(),
        }]
    }
}

#[derive(Default, Debug)]
pub struct Children {
    pub entities: Vec<crate::ecs::Entity>,
    /// Comma separated list of entity ids for editing in the inspector UI
    pub ids: String,
}

impl Component for Children {}
impl Inspectable for Children {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        // Update entity list from the current text field
        self.entities = self
            .ids
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .map(crate::ecs::Entity)
            .collect();

        vec![ExportedField {
            name: "children_ids",
            kind: ExportKind::Text,
            value: &mut self.ids as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<String>(),
        }]
    }
}

#[derive(Debug)]
pub enum Shape {
    Sphere {
        radius: f32,
    },
    Cube,
    Mesh {
        triangle_start_idx: u32,
        triangle_count: u32,
    },
}
impl Default for Shape {
    fn default() -> Self {
        Shape::Sphere { radius: 1.0 }
    }
}
impl Component for Shape {}

impl Inspectable for Shape {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        match self {
            Shape::Sphere { radius } => vec![ExportedField {
                name: "radius",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: radius as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            }],
            Shape::Cube => vec![],
            Shape::Mesh {
                triangle_start_idx,
                triangle_count,
            } => vec![
                ExportedField {
                    name: "triangle_start_idx",
                    kind: ExportKind::Slider {
                        min: 0.0,
                        max: 10000.0,
                    },
                    value: triangle_start_idx as *mut _ as *mut dyn std::any::Any,
                    type_id: std::any::TypeId::of::<u32>(),
                },
                ExportedField {
                    name: "triangle_count",
                    kind: ExportKind::Slider {
                        min: 0.0,
                        max: 10000.0,
                    },
                    value: triangle_count as *mut _ as *mut dyn std::any::Any,
                    type_id: std::any::TypeId::of::<u32>(),
                },
            ],
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Metadata {
    pub name: String,
    pub tags: Vec<String>,
}
impl Component for Metadata {}
impl Inspectable for Metadata {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "name",
            kind: ExportKind::Text,
            value: &mut self.name as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<String>(),
        }]
    }
    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut self.name);
        });
        ui.horizontal(|ui| {
            ui.label("Tags");
            let mut tag_str = self.tags.join(",");
            if ui.text_edit_singleline(&mut tag_str).changed() {
                self.tags = tag_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        });
    }
}

#[derive(Default, Debug)]
pub struct ScriptComponent {
    pub script: String,
}
impl Component for ScriptComponent {}
impl Inspectable for ScriptComponent {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "script",
            kind: ExportKind::Text,
            value: &mut self.script as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<String>(),
        }]
    }
}

#[derive(Default, Debug)]
pub struct LookAt {
    pub target: String,
    pub rotate_x: bool,
    pub rotate_y: bool,
    pub rotate_z: bool,
}
impl Component for LookAt {}
impl Inspectable for LookAt {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "target",
                kind: ExportKind::Text,
                value: &mut self.target as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<String>(),
            },
            ExportedField {
                name: "rotate_x",
                kind: ExportKind::Checkbox,
                value: &mut self.rotate_x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "rotate_y",
                kind: ExportKind::Checkbox,
                value: &mut self.rotate_y as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "rotate_z",
                kind: ExportKind::Checkbox,
                value: &mut self.rotate_z as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct Lifetime {
    pub remaining: f32,
}

impl Component for Lifetime {}

impl Default for Lifetime {
    fn default() -> Self {
        Self { remaining: 0.0 }
    }
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub velocity: [f32; 3],
    pub lifetime: f32,
    pub start_size: f32,
    pub end_size: f32,
    pub looping: bool,
    pub initial_lifetime: f32,
    pub initial_position: Option<[f32; 3]>,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            velocity: [0.0, 0.0, 0.0],
            lifetime: 1.0,
            start_size: 1.0,
            end_size: 1.0,
            looping: false,
            initial_lifetime: 1.0,
            initial_position: None,
        }
    }
}

impl Component for Particle {}

impl Inspectable for Particle {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "velocity_x",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.velocity[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "velocity_y",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.velocity[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "velocity_z",
                kind: ExportKind::Slider {
                    min: -50.0,
                    max: 50.0,
                },
                value: &mut self.velocity[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "lifetime",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.lifetime as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "start_size",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.start_size as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "end_size",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.end_size as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "looping",
                kind: ExportKind::Checkbox,
                value: &mut self.looping as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct CameraAttachment {
    pub fov: f32,
    /// Offset from the entity transform used when computing the camera
    /// position. Allows third-person style cameras.
    pub local_offset: [f32; 3],
}

impl Default for CameraAttachment {
    fn default() -> Self {
        Self {
            fov: 60.0_f32.to_radians(),
            local_offset: [0.0, 0.0, 0.0],
        }
    }
}

impl Component for CameraAttachment {}

impl Inspectable for CameraAttachment {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "fov",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 6.28,
                },
                value: &mut self.fov as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "offset_x",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.local_offset[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "offset_y",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.local_offset[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "offset_z",
                kind: ExportKind::Slider {
                    min: -10.0,
                    max: 10.0,
                },
                value: &mut self.local_offset[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct FreeFlightControls {
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub friction: f32,
    pub velocity: [f32; 3],
    pub yaw_velocity: f32,
    pub pitch_velocity: f32,
    pub angular_friction: f32,
}

impl Default for FreeFlightControls {
    fn default() -> Self {
        Self {
            yaw: -90.0,
            pitch: 0.0,
            speed: 5.0,
            sensitivity: 0.5,
            acceleration: 10.0,
            deceleration: 2.0,
            friction: 0.95,
            velocity: [0.0, 0.0, 0.0],
            yaw_velocity: 0.0,
            pitch_velocity: 0.0,
            angular_friction: 0.04,
        }
    }
}

impl Component for FreeFlightControls {}
impl Inspectable for FreeFlightControls {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "yaw",
                kind: ExportKind::Slider {
                    min: -180.0,
                    max: 180.0,
                },
                value: &mut self.yaw as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "pitch",
                kind: ExportKind::Slider {
                    min: -89.0,
                    max: 89.0,
                },
                value: &mut self.pitch as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "speed",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.speed as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "sensitivity",
                kind: ExportKind::Slider { min: 0.1, max: 5.0 },
                value: &mut self.sensitivity as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "acceleration",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 50.0,
                },
                value: &mut self.acceleration as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "deceleration",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 50.0,
                },
                value: &mut self.deceleration as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "friction",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.friction as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "angular_friction",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.angular_friction as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl Inspectable for Lifetime {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "remaining",
            kind: ExportKind::Slider {
                min: 0.0,
                max: 10.0,
            },
            value: &mut self.remaining as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<f32>(),
        }]
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Default for Anchor {
    fn default() -> Self {
        Self::TopLeft
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Default, Debug)]
pub struct UIScreenSpace;
impl Component for UIScreenSpace {}
impl Inspectable for UIScreenSpace {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug)]
pub struct UILabel {
    pub text: String,
    pub font_size: f32,
    pub color: [f32; 4],
    pub align: TextAlign,
    pub font_name: Option<String>,
}

impl Default for UILabel {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_size: 16.0,
            color: [255.0, 255.0, 255.0, 255.0],
            align: TextAlign::Left,
            font_name: None,
        }
    }
}

impl Component for UILabel {}
impl Inspectable for UILabel {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "text",
                kind: ExportKind::Text,
                value: &mut self.text as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<String>(),
            },
            ExportedField {
                name: "font_size",
                kind: ExportKind::Slider {
                    min: 8.0,
                    max: 128.0,
                },
                value: &mut self.font_size as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_r",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_g",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_b",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_a",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug, Default)]
pub struct UIPanel {
    pub color: [f32; 4],
    pub size: [f32; 2],
}

impl Component for UIPanel {}

impl Inspectable for UIPanel {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "color_r",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_g",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_b",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_a",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 1000.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 1000.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct UIButton {
    pub text: String,
    pub size: [f32; 2],
    pub clicked: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub text_color: [f32; 4],
    pub bg_color: [f32; 4],
}

impl Default for UIButton {
    fn default() -> Self {
        Self {
            text: String::new(),
            size: [80.0, 24.0],
            clicked: false,
            hovered: false,
            pressed: false,
            text_color: [0.0, 0.0, 0.0, 255.0],
            bg_color: [200.0, 200.0, 200.0, 255.0],
        }
    }
}

impl Component for UIButton {}

impl Inspectable for UIButton {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "text",
                kind: ExportKind::Text,
                value: &mut self.text as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<String>(),
            },
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 500.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 200.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "clicked",
                kind: ExportKind::Checkbox,
                value: &mut self.clicked as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "hovered",
                kind: ExportKind::Checkbox,
                value: &mut self.hovered as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "pressed",
                kind: ExportKind::Checkbox,
                value: &mut self.pressed as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "text_color_r",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.text_color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "text_color_g",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.text_color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "text_color_b",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.text_color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "text_color_a",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.text_color[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "bg_color_r",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.bg_color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "bg_color_g",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.bg_color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "bg_color_b",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.bg_color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "bg_color_a",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.bg_color[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct UITextEditor {
    pub text: String,
    pub size: [f32; 2],
    pub changed: bool,
    pub hovered: bool,
    pub focused: bool,
}

impl Default for UITextEditor {
    fn default() -> Self {
        Self {
            text: String::new(),
            size: [200.0, 24.0],
            changed: false,
            hovered: false,
            focused: false,
        }
    }
}

impl Component for UITextEditor {}

impl Inspectable for UITextEditor {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "text",
                kind: ExportKind::Text,
                value: &mut self.text as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<String>(),
            },
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 1000.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 200.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "changed",
                kind: ExportKind::Checkbox,
                value: &mut self.changed as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "hovered",
                kind: ExportKind::Checkbox,
                value: &mut self.hovered as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "focused",
                kind: ExportKind::Checkbox,
                value: &mut self.focused as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct UIList {
    pub items: Vec<String>,
    pub size: [f32; 2],
    pub edit_buffer: String,
    pub color: [f32; 4],
}

impl Default for UIList {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            size: [100.0, 100.0],
            edit_buffer: String::new(),
            color: [255.0, 255.0, 255.0, 255.0],
        }
    }
}

impl Component for UIList {}

impl Inspectable for UIList {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        self.items = self.edit_buffer.lines().map(|s| s.to_string()).collect();
        self.edit_buffer = self.items.join("\n");
        vec![
            ExportedField {
                name: "items",
                kind: ExportKind::Text,
                value: &mut self.edit_buffer as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<String>(),
            },
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 1000.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 10.0,
                    max: 1000.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_r",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_g",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_b",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_a",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[3] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug)]
pub struct UILayout {
    pub anchor: Anchor,
    pub offset: [f32; 2],
}

impl Default for UILayout {
    fn default() -> Self {
        Self {
            anchor: Anchor::TopLeft,
            offset: [0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sprite3D {
    pub texture: crate::rendering::texture::TextureHandle,
    pub size: [f32; 2],
    pub facing_camera: bool,
    pub double_sided: bool,
}

impl Default for Sprite3D {
    fn default() -> Self {
        Self {
            texture: crate::rendering::texture::TextureHandle::default(),
            size: [1.0, 1.0],
            facing_camera: true,
            double_sided: false,
        }
    }
}

impl Component for Sprite3D {}

impl Inspectable for Sprite3D {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "size_x",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 100.0,
                },
                value: &mut self.size[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "size_y",
                kind: ExportKind::Slider {
                    min: 0.1,
                    max: 100.0,
                },
                value: &mut self.size[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "facing_camera",
                kind: ExportKind::Checkbox,
                value: &mut self.facing_camera as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "double_sided",
                kind: ExportKind::Checkbox,
                value: &mut self.double_sided as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

impl Component for UILayout {}
impl Inspectable for UILayout {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "offset_x",
                kind: ExportKind::Slider {
                    min: -1000.0,
                    max: 1000.0,
                },
                value: &mut self.offset[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "offset_y",
                kind: ExportKind::Slider {
                    min: -1000.0,
                    max: 1000.0,
                },
                value: &mut self.offset[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ScoreValue {
    pub value: i32,
}

impl Component for ScoreValue {}

impl Inspectable for ScoreValue {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![ExportedField {
            name: "value",
            kind: ExportKind::Slider {
                min: 0.0,
                max: 1000.0,
            },
            value: &mut self.value as *mut _ as *mut dyn std::any::Any,
            type_id: std::any::TypeId::of::<i32>(),
        }]
    }
}

#[derive(Clone, Debug)]
pub struct Bloom {
    pub threshold: f32,
    pub intensity: f32,
    pub spread: f32,
    pub iterations: u8,
    pub tint: Vec3,
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            intensity: 0.8,
            spread: 2.0,
            iterations: 5,
            tint: Vec3::ONE,
        }
    }
}

impl Component for Bloom {}
impl Inspectable for Bloom {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "threshold",
                kind: ExportKind::Slider { min: 0.0, max: 5.0 },
                value: &mut self.threshold as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "intensity",
                kind: ExportKind::Slider { min: 0.0, max: 3.0 },
                value: &mut self.intensity as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "spread",
                kind: ExportKind::Slider { min: 0.5, max: 5.0 },
                value: &mut self.spread as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "iterations",
                kind: ExportKind::Slider {
                    min: 1.0,
                    max: 10.0,
                },
                value: &mut self.iterations as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<u8>(),
            },
            ExportedField {
                name: "tint_r",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.tint.x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "tint_g",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.tint.y as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "tint_b",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.tint.z as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Clone, Debug)]
pub struct DepthOfField {
    pub focal_depth: f32,
    pub focal_length: f32,
    pub fstop: f32,
    pub coc: f32,
    pub manual: bool,
    pub ndof_start: f32,
    pub ndof_dist: f32,
    pub fdof_start: f32,
    pub fdof_dist: f32,
    pub show_focus: bool,
    pub samples: u32,
    pub rings: u32,
    pub vignetting: bool,
    pub vign_out: f32,
    pub vign_in: f32,
    pub vign_fade: f32,
    pub autofocus: bool,
    pub focus: Vec2,
    pub max_blur: f32,
    pub threshold: f32,
    pub gain: f32,
    pub bias: f32,
    pub fringe: f32,
    pub noise: bool,
    pub namount: f32,
    pub depth_blur: bool,
    pub db_size: f32,
    pub pentagon: bool,
    pub feather: f32,
}

impl DepthOfField {
    pub fn aperture(&self) -> f32 {
        if self.fstop > 0.0 {
            self.focal_length / self.fstop
        } else {
            0.0
        }
    }
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            focal_depth: 5.0,
            focal_length: 50.0,
            fstop: 2.0,
            coc: 0.03,
            manual: false,
            ndof_start: 1.0,
            ndof_dist: 2.0,
            fdof_start: 1.0,
            fdof_dist: 3.0,
            show_focus: false,
            samples: 3,
            rings: 3,
            vignetting: false,
            vign_out: 1.3,
            vign_in: 0.0,
            vign_fade: 22.0,
            autofocus: false,
            focus: Vec2::new(0.5, 0.5),
            max_blur: 1.0,
            threshold: 0.7,
            gain: 100.0,
            bias: 0.5,
            fringe: 0.7,
            noise: true,
            namount: 0.0001,
            depth_blur: false,
            db_size: 1.25,
            pentagon: false,
            feather: 0.4,
        }
    }
}

impl Component for DepthOfField {}

impl Inspectable for DepthOfField {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        ui.add(egui::Slider::new(&mut self.focal_depth, 0.0..=100.0).text("Focal Depth"));
        ui.add(egui::Slider::new(&mut self.focal_length, 1.0..=200.0).text("Focal Length"));
        ui.add(egui::Slider::new(&mut self.fstop, 0.1..=32.0).text("f-stop"));
        ui.add(egui::Slider::new(&mut self.coc, 0.0..=5.0).text("CoC (mm)"));
        ui.checkbox(&mut self.manual, "Manual DoF");
        if self.manual {
            ui.add(egui::Slider::new(&mut self.ndof_start, 0.0..=10.0).text("Near Start"));
            ui.add(egui::Slider::new(&mut self.ndof_dist, 0.0..=10.0).text("Near Dist"));
            ui.add(egui::Slider::new(&mut self.fdof_start, 0.0..=10.0).text("Far Start"));
            ui.add(egui::Slider::new(&mut self.fdof_dist, 0.0..=10.0).text("Far Dist"));
        }
        ui.checkbox(&mut self.show_focus, "Show Focus");
        ui.add(egui::Slider::new(&mut self.samples, 1..=8).text("Samples"));
        ui.add(egui::Slider::new(&mut self.rings, 1..=8).text("Rings"));
        ui.add(egui::Slider::new(&mut self.max_blur, 0.0..=2.0).text("Max Blur"));
        ui.add(egui::Slider::new(&mut self.threshold, 0.0..=1.0).text("Threshold"));
        ui.add(egui::Slider::new(&mut self.gain, 0.0..=200.0).text("Gain"));
        ui.add(egui::Slider::new(&mut self.bias, 0.0..=1.0).text("Bias"));
        ui.add(egui::Slider::new(&mut self.fringe, 0.0..=2.0).text("Fringe"));
        ui.checkbox(&mut self.noise, "Noise Dither");
        ui.add(egui::Slider::new(&mut self.namount, 0.0..=0.01).text("Noise Amount"));
        ui.checkbox(&mut self.depth_blur, "Blur Depth Buffer");
        if self.depth_blur {
            ui.add(egui::Slider::new(&mut self.db_size, 0.0..=3.0).text("Depth Blur Size"));
        }
        ui.checkbox(&mut self.pentagon, "Pentagon Bokeh");
        if self.pentagon {
            ui.add(egui::Slider::new(&mut self.feather, 0.0..=1.0).text("Pentagon Feather"));
        }
        ui.checkbox(&mut self.vignetting, "Vignetting");
        if self.vignetting {
            ui.add(egui::Slider::new(&mut self.vign_out, 0.0..=2.0).text("Vignette Outer"));
            ui.add(egui::Slider::new(&mut self.vign_in, 0.0..=1.0).text("Vignette Inner"));
            ui.add(egui::Slider::new(&mut self.vign_fade, 0.0..=50.0).text("Vignette Fade"));
        }
        ui.checkbox(&mut self.autofocus, "Autofocus");
        if self.autofocus {
            ui.add(egui::Slider::new(&mut self.focus.x, 0.0..=1.0).text("Focus X"));
            ui.add(egui::Slider::new(&mut self.focus.y, 0.0..=1.0).text("Focus Y"));
        }
    }
}

#[derive(Clone, Debug)]
pub struct VolumetricFog {
    pub density: f32,
    pub color: [f32; 3],
}

impl Default for VolumetricFog {
    fn default() -> Self {
        Self {
            density: 0.0,
            color: [1.0, 1.0, 1.0],
        }
    }
}

impl Component for VolumetricFog {}
impl Inspectable for VolumetricFog {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "density",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.density as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_r",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_g",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_b",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    pub direction: [f32; 3],
    pub color: [f32; 3],
    pub intensity: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            color: [255.0, 255.0, 255.0],
            intensity: 1.0,
        }
    }
}

impl Component for DirectionalLight {}

impl Inspectable for DirectionalLight {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "direction_x",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.direction[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "direction_y",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.direction[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "direction_z",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.direction[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_r",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_g",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "color_b",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 255.0,
                },
                value: &mut self.color[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "intensity",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.intensity as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Clone, Debug)]
pub struct PostProcessing {
    pub bloom: Option<Bloom>,
    pub dof: Option<DepthOfField>,
    pub gi_quality: u32,
    pub gi_debug_mode: u32,
    pub gi_enabled: bool,
    pub path_traced_gi: bool,
    pub light_samples: u32,
    pub dir_light_samples: u32,
    pub max_bounces: u32,
    pub history_clamp_k: f32,
    pub temporal_blend: f32,
    pub gi_temporal_blend: f32,
    pub exposure: f32,
    pub auto_exposure: bool,
    pub atmosphere: bool,
}

impl Default for PostProcessing {
    fn default() -> Self {
        Self {
            bloom: None,
            dof: None,
            gi_quality: 0,
            gi_debug_mode: 0,
            gi_enabled: true,
            path_traced_gi: false,
            light_samples: 1,
            dir_light_samples: 1,
            max_bounces: 3,
            history_clamp_k: 1.5,
            // Higher values accumulate more history in the temporal filter
            temporal_blend: 1.0,
            gi_temporal_blend: 0.1,
            exposure: 1.0,
            auto_exposure: false,
            atmosphere: true,
        }
    }
}

impl Component for PostProcessing {}

impl Inspectable for PostProcessing {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }

    fn draw_ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Exposure");
        // Allow a wider exposure range so scenes can be brightened
        // sufficiently when default lighting feels too dim.
        ui.add(egui::Slider::new(&mut self.exposure, 0.0..=20.0));
        ui.checkbox(&mut self.auto_exposure, "Auto Exposure");
        ui.checkbox(&mut self.atmosphere, "Atmosphere");

        ui.collapsing("Global Illumination", |ui| {
            ui.checkbox(&mut self.gi_enabled, "Enabled");

            ui.label("GI Quality");
            egui::ComboBox::from_id_source("gi_quality_pp")
                .selected_text(match self.gi_quality {
                    0 => "Ultra",
                    1 => "High",
                    2 => "Low",
                    _ => "Off",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.gi_quality, 0, "Ultra");
                    ui.selectable_value(&mut self.gi_quality, 1, "High");
                    ui.selectable_value(&mut self.gi_quality, 2, "Low");
                    ui.selectable_value(&mut self.gi_quality, 3, "Off");
                });

            ui.checkbox(&mut self.path_traced_gi, "Path Traced GI");

            ui.label("GI Debug");
            egui::ComboBox::from_id_source("gi_debug_pp")
                .selected_text(match self.gi_debug_mode {
                    0 => "None",
                    1 => "Heatmap",
                    2 => "SDF Grid",
                    _ => "Cone Arrows",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.gi_debug_mode, 0, "None");
                    ui.selectable_value(&mut self.gi_debug_mode, 1, "Heatmap");
                    ui.selectable_value(&mut self.gi_debug_mode, 2, "SDF Grid");
                    ui.selectable_value(&mut self.gi_debug_mode, 3, "Cone Arrows");
                });

            ui.label("Light Samples");
            ui.add(egui::Slider::new(&mut self.light_samples, 1..=8));
            ui.label("Dir Light Samples");
            ui.add(egui::Slider::new(&mut self.dir_light_samples, 1..=8));
            ui.label("Max Bounces");
            ui.add(egui::Slider::new(&mut self.max_bounces, 1..=8));
        });

        ui.collapsing("Denoiser", |ui| {
            ui.label("History Clamp");
            ui.add(egui::Slider::new(&mut self.history_clamp_k, 0.0..=10.0));
            ui.label("Frame Blend");
            ui.add(egui::Slider::new(&mut self.temporal_blend, 0.0..=10.0));
            ui.label("GI Blend");
            ui.add(egui::Slider::new(&mut self.gi_temporal_blend, 0.0..=1.0));
        });

        ui.collapsing("Depth of Field", |ui| {
            let mut enabled = self.dof.is_some();
            if ui.checkbox(&mut enabled, "Enabled").changed() {
                if enabled && self.dof.is_none() {
                    self.dof = Some(DepthOfField::default());
                } else if !enabled {
                    self.dof = None;
                }
            }
            if let Some(d) = &mut self.dof {
                d.draw_ui(ui);
            }
        });

        ui.collapsing("Bloom", |ui| {
            let mut enabled = self.bloom.is_some();
            if ui.checkbox(&mut enabled, "Enabled").changed() {
                if enabled && self.bloom.is_none() {
                    self.bloom = Some(Bloom::default());
                } else if !enabled {
                    self.bloom = None;
                }
            }
            if let Some(b) = &mut self.bloom {
                b.draw_ui(ui);
            }
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPlayState {
    Stopped,
    Playing,
    Paused,
}

pub type AudioClipHandle = String;

#[derive(Debug)]
pub struct AudioSource {
    pub clip: Option<AudioClipHandle>,
    pub volume: f32,
    pub pitch: f32,
    pub loop_: bool,
    pub play_on_start: bool,
    pub spatial: bool,
    pub state: AudioPlayState,
}

#[derive(Debug, Clone, Copy)]
pub struct Raycast {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub max_distance: f32,
    /// Entity to ignore when computing intersections.
    pub ignore_entity: crate::ecs::Entity,
    pub hit_distance: f32,
    pub hit_position: [f32; 3],
    pub hit_entity: crate::ecs::Entity,
}

impl Component for Raycast {}

impl Default for Raycast {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            direction: [0.0, -1.0, 0.0],
            max_distance: 10.0,
            ignore_entity: crate::ecs::Entity(0),
            hit_distance: 0.0,
            hit_position: [0.0; 3],
            hit_entity: crate::ecs::Entity(0),
        }
    }
}

impl Inspectable for Raycast {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "origin_x",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.origin[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "origin_y",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.origin[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "origin_z",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.origin[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            clip: None,
            volume: 1.0,
            pitch: 1.0,
            loop_: false,
            play_on_start: false,
            spatial: false,
            state: AudioPlayState::Stopped,
        }
    }
}

impl Component for AudioSource {}

impl Inspectable for AudioSource {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        if self.clip.is_none() {
            self.clip = Some(String::new());
        }
        let clip = self.clip.as_mut().unwrap();
        vec![
            ExportedField {
                name: "clip",
                kind: ExportKind::Text,
                value: clip as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<String>(),
            },
            ExportedField {
                name: "volume",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.volume as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "pitch",
                kind: ExportKind::Slider { min: 0.5, max: 2.0 },
                value: &mut self.pitch as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "loop",
                kind: ExportKind::Checkbox,
                value: &mut self.loop_ as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "play_on_start",
                kind: ExportKind::Checkbox,
                value: &mut self.play_on_start as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "spatial",
                kind: ExportKind::Checkbox,
                value: &mut self.spatial as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
        ]
    }
}

/// Buffer of pending input packets for a networked entity.
#[derive(Default, Debug)]
pub struct InputBuffer {
    /// Raw input packets queued for this entity.
    pub inputs: VecDeque<crate::net::InputData>,
}

impl Component for InputBuffer {}
impl Inspectable for InputBuffer {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        Vec::new()
    }
}

/// Marker for entities replicated by [`UnreliableSyncSystem`].
#[derive(Default, Debug, Clone, Copy)]
pub struct UnreliableSync;

impl Component for UnreliableSync {}
impl Inspectable for UnreliableSync {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LerpState {
    PlayingForward,
    PlayingBackward,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    None,
    Loop,
    PingPong,
}

#[derive(Clone, Copy)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Custom(fn(f32) -> f32),
}

impl std::fmt::Debug for Easing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Easing::Linear => f.write_str("Linear"),
            Easing::EaseIn => f.write_str("EaseIn"),
            Easing::EaseOut => f.write_str("EaseOut"),
            Easing::EaseInOut => f.write_str("EaseInOut"),
            Easing::Custom(_) => f.write_str("Custom"),
        }
    }
}

impl Default for Easing {
    fn default() -> Self {
        Easing::Linear
    }
}

pub trait Interpolate: Copy {
    fn lerp(a: Self, b: Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}

impl Interpolate for [f32; 3] {
    fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    }
}

#[derive(Debug)]
pub struct LerpData<T: Interpolate + Default> {
    pub start: T,
    pub end: T,
    pub progress: f32,
    pub speed: f32,
    pub loop_mode: LoopMode,
    pub state: LerpState,
    pub easing: Easing,
}

impl<T: Interpolate + Default> Default for LerpData<T> {
    fn default() -> Self {
        Self {
            start: T::default(),
            end: T::default(),
            progress: 0.0,
            speed: 1.0,
            loop_mode: LoopMode::None,
            state: LerpState::Stopped,
            easing: Easing::Linear,
        }
    }
}

fn easing_value(t: f32, easing: &Easing) -> f32 {
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t,
        Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        Easing::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - 2.0 * (1.0 - t) * (1.0 - t)
            }
        }
        Easing::Custom(f) => f(t),
    }
}

impl LerpData<f32> {
    pub fn value(&self) -> f32 {
        let t = easing_value(self.progress.clamp(0.0, 1.0), &self.easing);
        Interpolate::lerp(self.start, self.end, t)
    }
}

impl LerpData<[f32; 3]> {
    pub fn value(&self) -> [f32; 3] {
        let t = easing_value(self.progress.clamp(0.0, 1.0), &self.easing);
        Interpolate::lerp(self.start, self.end, t)
    }
}

impl Inspectable for LerpData<f32> {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "start",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.start as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "end",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.end as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "progress",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.progress as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "speed",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.speed as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

impl Inspectable for LerpData<[f32; 3]> {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "start_x",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.start[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "start_y",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.start[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "start_z",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.start[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "end_x",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.end[0] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "end_y",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.end[1] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "end_z",
                kind: ExportKind::Slider {
                    min: -100.0,
                    max: 100.0,
                },
                value: &mut self.end[2] as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "progress",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.progress as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "speed",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.speed as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}

#[derive(Debug)]
pub enum Lerp {
    F32(LerpData<f32>),
    Vec3(LerpData<[f32; 3]>),
}

impl Default for Lerp {
    fn default() -> Self {
        Lerp::F32(LerpData::default())
    }
}

impl Component for Lerp {}

impl Lerp {
    pub fn value_f32(&self) -> Option<f32> {
        match self {
            Lerp::F32(l) => Some(l.value()),
            _ => None,
        }
    }

    pub fn value_vec3(&self) -> Option<[f32; 3]> {
        match self {
            Lerp::Vec3(l) => Some(l.value()),
            _ => None,
        }
    }
}

impl Inspectable for Lerp {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        match self {
            Lerp::F32(l) => l.exported_fields_mut(),
            Lerp::Vec3(l) => l.exported_fields_mut(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Animation {
    pub clip: String,
    pub time: f32,
    pub playing: bool,
    pub translation_scale: f32, // Scale factor for translation animations
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            clip: String::new(),
            time: 0.0,
            playing: true,
            translation_scale: 1.0, // Default to normal scale
        }
    }
}

impl Component for Animation {}

impl Inspectable for Animation {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug, Clone)]
pub struct MorphTargets {
    pub morph_key: String, // Key to look up morph targets in AssetManager
}

impl Default for MorphTargets {
    fn default() -> Self {
        Self {
            morph_key: String::new(),
        }
    }
}

impl Component for MorphTargets {}

impl Inspectable for MorphTargets {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug, Clone)]
pub struct MorphWeights {
    pub weights: Vec<f32>, // Current blend weights for each morph target
}

impl Default for MorphWeights {
    fn default() -> Self {
        Self {
            weights: Vec::new(),
        }
    }
}

impl Component for MorphWeights {}

impl Inspectable for MorphWeights {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug, Clone)]
pub struct Skin {
    pub inverse_bind_mats: Vec<[[f32; 4]; 4]>,
    pub joints: Vec<crate::ecs::Entity>,
}

impl Component for Skin {}

impl Inspectable for Skin {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![]
    }
}

#[derive(Debug)]
pub struct Timer {
    pub autostart: bool,
    pub one_shot: bool,
    pub paused: bool,
    pub wait_time: f32,
    pub time_left: f32,
    pub started: bool,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            autostart: false,
            one_shot: false,
            paused: false,
            wait_time: 1.0,
            time_left: 1.0,
            started: false,
        }
    }
}

impl Component for Timer {}

impl Timer {
    pub fn start(&mut self) {
        self.started = true;
        self.paused = false;
        self.time_left = self.wait_time;
    }

    pub fn stop(&mut self) {
        self.started = false;
    }

    pub fn is_stopped(&self) -> bool {
        !self.started
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if self.autostart && !self.started {
            self.start();
        }
        if !self.started || self.paused {
            return false;
        }
        self.time_left -= dt;
        if self.time_left <= 0.0 {
            if self.one_shot {
                self.stop();
            } else {
                self.time_left += self.wait_time;
            }
            return true;
        }
        false
    }
}

impl Inspectable for Timer {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "autostart",
                kind: ExportKind::Checkbox,
                value: &mut self.autostart as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "one_shot",
                kind: ExportKind::Checkbox,
                value: &mut self.one_shot as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "paused",
                kind: ExportKind::Checkbox,
                value: &mut self.paused as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<bool>(),
            },
            ExportedField {
                name: "wait_time",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.wait_time as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "time_left",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 10.0,
                },
                value: &mut self.time_left as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
        ]
    }
}
#[derive(Debug)]
pub struct Atmosphere {
    pub planet_radius: f32,
    pub atmo_radius: f32,
    pub ray_beta: Vec3,
    pub mie_beta: Vec3,
    pub ambient_beta: Vec3,
    pub absorption_beta: Vec3,
    pub g: f32,
    pub height_ray: f32,
    pub height_mie: f32,
    pub height_absorption: f32,
    pub absorption_falloff: f32,
    pub primary_steps: i32,
    pub light_steps: i32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            planet_radius: 100.0,
            atmo_radius: 110.0,
            ray_beta: Vec3::new(5.5e-3, 0.013, 0.0224),
            mie_beta: Vec3::splat(0.021),
            ambient_beta: Vec3::ZERO,
            absorption_beta: Vec3::new(0.0204, 0.0497, 0.00195),
            g: 0.7,
            height_ray: 8.0,
            height_mie: 1.2,
            height_absorption: 30.0,
            absorption_falloff: 4.0,
            primary_steps: 16,
            light_steps: 8,
        }
    }
}

impl Component for Atmosphere {}

impl Inspectable for Atmosphere {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {
        vec![
            ExportedField {
                name: "planet_radius",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 1000.0,
                },
                value: &mut self.planet_radius as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "atmo_radius",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 1000.0,
                },
                value: &mut self.atmo_radius as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "ray_beta_r",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.ray_beta.x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "ray_beta_g",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.ray_beta.y as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "ray_beta_b",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.ray_beta.z as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "mie_beta",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.mie_beta.x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "ambient_beta",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.ambient_beta.x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "absorption_beta_r",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.absorption_beta.x as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "absorption_beta_g",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.absorption_beta.y as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "absorption_beta_b",
                kind: ExportKind::Slider { min: 0.0, max: 1.0 },
                value: &mut self.absorption_beta.z as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "g",
                kind: ExportKind::Slider {
                    min: -1.0,
                    max: 1.0,
                },
                value: &mut self.g as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "height_ray",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.height_ray as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "height_mie",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.height_mie as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "height_absorption",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.height_absorption as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "absorption_falloff",
                kind: ExportKind::Slider {
                    min: 0.0,
                    max: 100.0,
                },
                value: &mut self.absorption_falloff as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<f32>(),
            },
            ExportedField {
                name: "primary_steps",
                kind: ExportKind::Slider {
                    min: 1.0,
                    max: 64.0,
                },
                value: &mut self.primary_steps as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<i32>(),
            },
            ExportedField {
                name: "light_steps",
                kind: ExportKind::Slider {
                    min: 1.0,
                    max: 64.0,
                },
                value: &mut self.light_steps as *mut _ as *mut dyn std::any::Any,
                type_id: std::any::TypeId::of::<i32>(),
            },
        ]
    }
}
