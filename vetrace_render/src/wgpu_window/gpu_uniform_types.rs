use super::*;

// Uniform layouts shared by WGPU render passes.

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct CameraUniform {
    pub(super) view_proj: [[f32; 4]; 4],
    pub(super) camera_position: [f32; 4],
    pub(super) camera_forward: [f32; 4],
    pub(super) inverse_view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct EvsmPassUniform {
    /// x = horizontal blur direction, y = vertical blur direction, z = radius texels, w = cascade layer.
    pub(super) direction_radius_layer: [f32; 4],
    /// x = EVSM exponent, y = shadow map size, z/w reserved.
    pub(super) exponent_size: [f32; 4],
}

#[derive(Clone, Copy)]
pub(super) struct GpuSurfaceConfig {
    pub(super) format: wgpu::TextureFormat,
}


#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct SsaoUniform {
    /// x = width, y = height, z = radius pixels, w = intensity.
    pub(super) params0: [f32; 4],
    /// x = depth bias, y = sample count, z = near, w = far.
    pub(super) params1: [f32; 4],
    /// x = blur radius pixels, yzw reserved.
    pub(super) params2: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct CustomPostProcessUniform {
    /// Eight vec4 parameter slots. Game code fills `CustomPostProcessPass::params`
    /// as a flat f32 list; the backend packs up to the first 32 values here.
    pub(super) params: [[f32; 4]; 8],
    /// x = width, y = height, z = time_seconds, w = pass_index.
    pub(super) screen_time: [f32; 4],
    /// x = param_count, y = input mode, z/w reserved.
    pub(super) info: [f32; 4],
    /// Camera data is appended so advanced full-screen effects such as SSR can
    /// reconstruct world positions from the sampled scene depth. Older custom
    /// shaders may keep declaring only the prefix above; a larger bound uniform
    /// buffer remains ABI-compatible with their shorter WGSL struct.
    pub(super) view_proj: [[f32; 4]; 4],
    pub(super) inverse_view_proj: [[f32; 4]; 4],
    pub(super) camera_position: [f32; 4],
    pub(super) camera_forward: [f32; 4],
    /// Previous frame camera transform for temporal full-screen effects.
    pub(super) previous_view_proj: [[f32; 4]; 4],
}
