use glam::Vec3;
use serde::{Deserialize, Serialize};
use vetrace_render::Material;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneMaterial {
    pub base_color: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub alpha: f32,
    #[serde(default)]
    pub base_color_texture_path: Option<String>,
    #[serde(default = "default_uv_scale")]
    pub uv_scale: [f32; 2],
}

impl SceneMaterial {
    pub fn from_material(material: &Material) -> Self {
        Self {
            base_color: material.base_color.to_array(),
            roughness: material.roughness,
            metallic: material.metallic,
            alpha: material.alpha,
            base_color_texture_path: material.base_color_texture_path.clone(),
            uv_scale: material.uv_scale.to_array(),
        }
    }

    pub fn base_color_vec3(&self) -> Vec3 { Vec3::from_array(self.base_color).clamp(Vec3::ZERO, Vec3::ONE) }

    pub fn to_material(&self) -> Material {
        Material {
            base_color: self.base_color_vec3(),
            roughness: self.roughness,
            metallic: self.metallic,
            alpha: self.alpha,
            base_color_texture_path: self.base_color_texture_path.clone(),
            uv_scale: glam::Vec2::from_array(self.uv_scale).max(glam::Vec2::splat(0.0001)),
            ..Material::default()
        }
    }
}

impl Default for SceneMaterial {
    fn default() -> Self {
        Self { base_color: [0.45, 0.55, 0.70], roughness: 0.75, metallic: 0.0, alpha: 1.0, base_color_texture_path: None, uv_scale: default_uv_scale() }
    }
}

fn default_uv_scale() -> [f32; 2] { [1.0, 1.0] }
