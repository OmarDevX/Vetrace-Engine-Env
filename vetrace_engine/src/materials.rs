use crate::gpu::TextureHandle;

#[derive(Clone, Debug)]
pub struct PbrMaterial {
    pub name: String,
    pub base_color: [f32; 4], // linear RGBA
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    /// Optional override for the dielectric F0 term. If all components are
    /// zero the IOR value will be used instead.
    pub specular_f0: [f32; 3],
    pub ior: f32,
    pub opacity: f32,
    pub base_color_tex: Option<TextureHandle>,
    pub metallic_roughness_tex: Option<TextureHandle>,
    pub normal_tex: Option<TextureHandle>,
    pub occlusion_tex: Option<TextureHandle>,
    pub emissive_tex: Option<TextureHandle>,
}