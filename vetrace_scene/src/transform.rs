use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use vetrace_core::Transform;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl SceneTransform {
    pub fn from_transform(transform: &Transform) -> Self {
        Self {
            translation: transform.translation.to_array(),
            rotation: transform.rotation.to_array(),
            scale: transform.scale.to_array(),
        }
    }

    pub fn to_transform(&self) -> Transform {
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: self.rotation_quat(),
            scale: self.scale_vec3(),
        }
    }

    pub fn translation_vec3(&self) -> Vec3 { Vec3::from_array(self.translation) }
    pub fn rotation_quat(&self) -> Quat {
        let q = Quat::from_array(self.rotation);
        if q.length_squared().is_finite() && q.length_squared() > 1.0e-8 { q.normalize() } else { Quat::IDENTITY }
    }
    pub fn scale_vec3(&self) -> Vec3 { Vec3::from_array(self.scale).max(Vec3::splat(0.001)) }
}

impl Default for SceneTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO.to_array(),
            rotation: Quat::IDENTITY.to_array(),
            scale: Vec3::ONE.to_array(),
        }
    }
}
