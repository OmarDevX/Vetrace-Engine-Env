use super::*;
use std::sync::Arc;

// Split-out implementation details for `wgpu_window.rs`.

#[derive(Clone, Debug)]
pub(super) enum PipelineKind {
    Default,
    DefaultDoubleSided,
    Transparent,
    TransparentDoubleSided,
    Custom { key: String, bucket: CustomShaderRenderBucket },
    OutlineMask,
    OutlineOverlay,
}

pub(super) struct PreparedOutlineDraw {
    pub(super) mask: PreparedDraw,
    pub(super) outline: PreparedDraw,
}

#[derive(Clone)]
pub(super) struct PreparedGeometryBuffers {
    pub(super) vertex_buffer: Arc<wgpu::Buffer>,
    pub(super) index_buffer: Option<Arc<wgpu::Buffer>>,
    pub(super) vertex_count: u32,
    pub(super) index_count: u32,
}

impl PreparedGeometryBuffers {
    pub(super) fn draw_count(&self) -> u32 {
        if self.index_buffer.is_some() { self.index_count } else { self.vertex_count }
    }
}

pub(super) struct PreparedShadowDraw {
    pub(super) vertex_buffer: Arc<wgpu::Buffer>,
    pub(super) index_buffer: Option<Arc<wgpu::Buffer>>,
    pub(super) material_bind_group: wgpu::BindGroup,
    pub(super) vertex_count: u32,
    pub(super) index_count: u32,
    pub(super) bounds_min: Vec3,
    pub(super) bounds_max: Vec3,
}

pub(super) struct PreparedDraw {
    pub(super) vertex_buffer: Arc<wgpu::Buffer>,
    pub(super) index_buffer: Option<Arc<wgpu::Buffer>>,
    pub(super) material_bind_group: Arc<wgpu::BindGroup>,
    pub(super) vertex_count: u32,
    pub(super) index_count: u32,
    pub(super) pipeline: PipelineKind,
    pub(super) sort_depth: f32,
}

pub(super) struct PendingDraw<'a> {
    pub(super) object: &'a RenderObject,
    pub(super) geometry: IndexedGeometry,
    pub(super) geometry_key: u64,
    pub(super) geometry_signature: GeometryBufferSignature,
    pub(super) pipeline: PipelineKind,
    pub(super) use_custom_material: bool,
    pub(super) sort_depth: f32,
    pub(super) bounds_min: Vec3,
    pub(super) bounds_max: Vec3,
}

pub(super) struct IndexedGeometry {
    pub(super) vertices: Vec<GpuVertex>,
    pub(super) indices: Option<Vec<u32>>,
}

impl IndexedGeometry {
    pub(super) fn draw_count(&self) -> usize {
        self.indices.as_ref().map(|indices| indices.len()).unwrap_or(self.vertices.len())
    }
}

pub(super) struct CachedGeometryBuffers {
    pub(super) signature: GeometryBufferSignature,
    pub(super) vertex_buffer: Arc<wgpu::Buffer>,
    pub(super) index_buffer: Option<Arc<wgpu::Buffer>>,
    pub(super) vertex_count: u32,
    pub(super) index_count: u32,
    pub(super) last_used_frame: u64,
}

pub(super) struct CachedSceneDraw {
    pub(super) signature: SceneDrawSignature,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) material_bind_group: Arc<wgpu::BindGroup>,
    pub(super) last_used_frame: u64,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SceneDrawSignature {
    pub(super) mesh_id: u64,
    pub(super) shape_kind: u64,
    pub(super) vertex_count: u32,
    pub(super) index_count: u32,
    pub(super) material_textures: [u64; 6],
    pub(super) render_textures_hash: u64,
    pub(super) pipeline_kind: u64,
    pub(super) extra: u32,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GeometryBufferSignature {
    pub(super) mesh_id: u64,
    pub(super) shape_kind: u64,
    pub(super) vertex_count: u32,
    pub(super) index_count: u32,
    pub(super) extra: u32,
    pub(super) outline_scale: [u32; 3],
    pub(super) geometry_revision: u64,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct ShadowCandidate {
    pub(super) index: usize,
    pub(super) priority: u8,
    pub(super) distance2: f32,
    pub(super) vertices: usize,
    pub(super) bounds_min: Vec3,
    pub(super) bounds_max: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ShadowInfo {
    pub(super) enabled: bool,
    pub(super) soft_radius: f32,
    pub(super) view_proj: [Mat4; SHADOW_CASCADE_COUNT],
    pub(super) cascade_splits: [f32; SHADOW_CASCADE_COUNT],
    pub(super) cascade_count: usize,
    pub(super) bias: f32,
    pub(super) slope_bias: f32,
    pub(super) normal_bias: f32,
    pub(super) pcf_quality: f32,
    pub(super) filter_mode: ShadowFilterMode,
    pub(super) pcss_light_radius: f32,
    pub(super) evsm_blur_radius: f32,
    pub(super) evsm_exponent: f32,
}

