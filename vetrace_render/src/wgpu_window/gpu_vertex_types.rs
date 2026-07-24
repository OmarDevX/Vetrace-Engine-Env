use super::*;

// GPU vertex layouts used by the scene and overlay pipelines.

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct GpuVertex {
    pub(super) position: [f32; 3],
    pub(super) normal: [f32; 3],
    pub(super) uv: [f32; 2],
    pub(super) color: [f32; 4],
    pub(super) tangent: [f32; 4],
    pub(super) lightmap_uv: [f32; 2],
}

impl GpuVertex {
    pub(super) const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4, 4 => Float32x4, 5 => Float32x2];

    pub(super) fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct OverlayVertex {
    pub(super) position: [f32; 2],
    pub(super) color: [f32; 4],
}

impl OverlayVertex {
    pub(super) const ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];

    pub(super) fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<OverlayVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}
