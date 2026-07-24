use super::*;

// WGPU formats and renderer capacity limits.

pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
pub(super) const SHADOW_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const EVSM_MOMENT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(super) const SSAO_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub(super) const DEFAULT_SHADOW_MAP_SIZE: u32 = 1024;
pub(super) const SHADOW_CASCADE_COUNT: usize = 4;
pub(super) const MAX_DIRECTIONAL_LIGHTS: usize = 4;
pub(super) const MAX_POINT_LIGHTS: usize = 8;
pub(super) const MAX_SPOT_LIGHTS: usize = 4;
