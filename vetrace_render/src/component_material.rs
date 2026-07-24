use super::*;

/// glTF-compatible alpha handling. `Mask` keeps depth writes and discards
/// pixels below `Material::alpha_cutoff`; `Blend` is rendered in the transparent
/// pass with depth writes disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

impl Default for AlphaMode {
    fn default() -> Self { Self::Opaque }
}

/// Lightweight PBR-style material data. Backend-specific GPU material types
/// should live beside the actual renderer implementation, not in core.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Material {
    pub base_color: Vec3,
    /// Optional base-color/albedo texture. Renderers that do not support textures
    /// simply ignore this and keep using `base_color`.
    pub base_color_texture: Option<TextureHandle>,
    /// Optional source path used by editor/scene tooling to reload the runtime
    /// texture handle. Runtime renderers use `base_color_texture`.
    #[serde(default)]
    pub base_color_texture_path: Option<String>,
    /// Multiplier applied to material UVs. Values above 1 repeat textures more;
    /// values below 1 make the texture appear larger. X/Y can differ so editor
    /// texture tools can preserve the imported image aspect ratio while tiling.
    /// The sampler uses Repeat.
    #[serde(default = "default_uv_scale")]
    pub uv_scale: Vec2,
    /// Optional tangent-space normal map. Uses glTF convention where +Y is up.
    pub normal_texture: Option<TextureHandle>,
    /// Optional packed glTF metallic-roughness texture: G = roughness, B = metallic.
    pub metallic_roughness_texture: Option<TextureHandle>,
    /// Optional occlusion map. glTF stores occlusion in the red channel.
    pub occlusion_texture: Option<TextureHandle>,
    /// Optional emissive texture multiplied by `emissive`.
    pub emissive_texture: Option<TextureHandle>,
    pub emissive: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub alpha: f32,
    /// glTF alpha mode. Opaque and Mask draw in the opaque pass; Blend draws
    /// after opaque objects and is sorted back-to-front by the WGPU backend.
    pub alpha_mode: AlphaMode,
    /// glTF alpha cutoff for `AlphaMode::Mask`. Default glTF cutoff is 0.5.
    pub alpha_cutoff: f32,
    /// glTF `doubleSided`. When true, the WGPU backend disables back-face
    /// culling for this material.
    pub double_sided: bool,
    pub normal_scale: f32,
    pub occlusion_strength: f32,
    pub is_glass: bool,
    pub specular_f0: Vec3,
    pub ior: f32,
}

fn default_uv_scale() -> Vec2 { Vec2::ONE }

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: Vec3::ONE,
            base_color_texture: None,
            base_color_texture_path: None,
            uv_scale: Vec2::ONE,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            emissive: Vec3::ZERO,
            roughness: 0.5,
            metallic: 0.0,
            alpha: 1.0,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            // Keep engine-authored primitives backward-compatible. glTF imports
            // explicitly set this from the file, where the glTF default is false.
            double_sided: true,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            is_glass: false,
            specular_f0: Vec3::ZERO,
            ior: 1.5,
        }
    }
}
