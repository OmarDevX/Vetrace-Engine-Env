use crate::gpu::TextureHandle;

pub const MATERIAL_TAG_NEEDS_ACCURATE_REFLECTION: u32 = 1 << 0;
pub const MATERIAL_TAG_CAN_USE_PROBE: u32 = 1 << 1;
pub const MATERIAL_TAG_RASTER_ONLY: u32 = 1 << 2;
pub const MATERIAL_TAG_TRANSPARENT_EXPENSIVE: u32 = 1 << 3;
pub const MATERIAL_TAG_EMISSIVE_STATIC: u32 = 1 << 4;

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
    /// Bitmask of MATERIAL_TAG_* hints used by fallback policies.
    pub fallback_tags: u32,
}

impl PbrMaterial {
    pub fn with_fallback_tag(mut self, tag: u32) -> Self {
        self.fallback_tags |= tag;
        self
    }
}
