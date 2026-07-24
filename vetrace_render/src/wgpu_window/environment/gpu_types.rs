use super::*;

// Keep the cube-array below the common 256-array-layer limit: one fallback
// cube, two global transition cubes, and two cubes for each local probe.
pub(super) const MAX_REFLECTION_PROBES: usize = 16;
pub(super) const ENVIRONMENT_CUBEMAP_CAPACITY: u32 = 1 + ENVIRONMENT_STATIC_SLOT_COUNT + MAX_REFLECTION_PROBES as u32 * 2;
pub(super) const ENVIRONMENT_CUBEMAP_FACE_SIZE: u32 = 256;
pub(super) const ENVIRONMENT_CUBEMAP_MIP_COUNT: u32 = 9;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct EnvironmentUniform {
    /// x = global primary slot, y = global secondary slot,
    /// z = uploaded reflection probe count, w = cubemap mip count.
    pub(super) slots_counts: [u32; 4],
    /// x = global transition, y = intensity, z = Y rotation radians,
    /// w = environment enabled.
    pub(super) params0: [f32; 4],
    /// x = draw sky, y = diffuse IBL, z = specular IBL, w = linear HDR capture view.
    pub(super) params1: [f32; 4],
    /// x = exposure, y = gamma, z = tone mapper, w = reserved.
    pub(super) post_process: [f32; 4],
}

impl Default for EnvironmentUniform {
    fn default() -> Self {
        Self {
            slots_counts: [0, 0, 0, ENVIRONMENT_CUBEMAP_MIP_COUNT],
            params0: [0.0, 1.0, 0.0, 0.0],
            params1: [0.0, 0.0, 0.0, 0.0],
            post_process: [1.0, 2.2, 1.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct GpuReflectionProbe {
    pub(super) world_to_probe: [[f32; 4]; 4],
    /// xyz = local-space half extents, w = blend distance.
    pub(super) half_extents_blend: [f32; 4],
    /// xyz = local-space capture position, w = intensity.
    pub(super) capture_intensity: [f32; 4],
    /// x = primary slot, y = secondary slot, z = parallax mode,
    /// w = priority bit pattern (reserved for shader-side priority policies).
    pub(super) slots_modes: [u32; 4],
    /// x = cubemap transition, yzw reserved.
    pub(super) transition_params: [f32; 4],
    /// x = included render layers, y = excluded render layers, zw reserved.
    pub(super) layer_masks: [u32; 4],
}

pub(super) struct GpuEnvironmentCubemapPool {
    pub(super) texture: wgpu::Texture,
    pub(super) slots: HashMap<u64, u32>,
    pub(super) signature: u64,
}


pub(super) struct GpuEnvironmentBrdfLut {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) sampler: wgpu::Sampler,
}

pub(super) struct InitialEnvironmentResources {
    pub(super) cubemap_pool: GpuEnvironmentCubemapPool,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) probe_buffer: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) capture_uniform_buffer: wgpu::Buffer,
    pub(super) capture_bind_group: wgpu::BindGroup,
    pub(super) prefilter_layout: wgpu::BindGroupLayout,
    pub(super) prefilter_pipeline: wgpu::RenderPipeline,
    pub(super) brdf_lut: GpuEnvironmentBrdfLut,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CachedReflectionProbeSelection {
    pub(super) entities: [u64; 4],
    pub(super) count: usize,
    pub(super) last_used_frame: u64,
}

impl Default for CachedReflectionProbeSelection {
    fn default() -> Self {
        Self {
            entities: [u64::MAX; 4],
            count: 0,
            last_used_frame: 0,
        }
    }
}

pub(super) const ENVIRONMENT_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(super) const ENVIRONMENT_STATIC_SLOT_COUNT: u32 = 9;
pub(super) const ENVIRONMENT_RUNTIME_SLOT_BASE: u32 = 1 + ENVIRONMENT_STATIC_SLOT_COUNT;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct ReflectionPrefilterUniform {
    pub(super) face_sample_count: [u32; 4],
    pub(super) params: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub(super) enum ReflectionCapturePhase {
    Idle,
    Capturing { next_face: u32, write_index: usize },
    Filtering { next_mip: u32, write_index: usize },
}

pub(super) struct GpuReflectionCaptureTarget {
    pub(super) _texture: wgpu::Texture,
    pub(super) _cube_view: wgpu::TextureView,
    pub(super) face_views: Vec<wgpu::TextureView>,
    pub(super) depth: DepthTarget,
    pub(super) camera_buffers: Vec<wgpu::Buffer>,
    pub(super) camera_bind_groups: Vec<wgpu::BindGroup>,
    /// Six independent sets of cascade camera buffers, one set per cube face.
    pub(super) shadow_camera_buffers: Vec<wgpu::Buffer>,
    pub(super) shadow_camera_bind_groups: Vec<wgpu::BindGroup>,
    pub(super) prefilter_uniform_buffers: Vec<wgpu::Buffer>,
    pub(super) prefilter_bind_groups: Vec<wgpu::BindGroup>,
    pub(super) resolution: u32,
}

pub(super) struct ReflectionProbeCaptureState {
    pub(super) target: GpuReflectionCaptureTarget,
    pub(super) slots: [u32; 2],
    pub(super) front_index: usize,
    pub(super) has_capture: bool,
    pub(super) transition_to: Option<usize>,
    pub(super) transition_started: Option<Instant>,
    pub(super) transition_duration: f32,
    pub(super) initial_transition_started: Option<Instant>,
    pub(super) phase: ReflectionCapturePhase,
    pub(super) completed_revision: u32,
    pub(super) in_progress_revision: u32,
    pub(super) completed_scene_signature: u64,
    pub(super) in_progress_scene_signature: u64,
    pub(super) observed_scene_signature: u64,
    pub(super) scene_change_observed_at: Option<Instant>,
    pub(super) last_completed: Option<Instant>,
}

impl ReflectionProbeCaptureState {
    pub(super) fn observe_scene_signature(&mut self, signature: u64, now: Instant) {
        if self.completed_scene_signature == signature {
            self.observed_scene_signature = signature;
            self.scene_change_observed_at = None;
        } else if self.observed_scene_signature != signature {
            self.observed_scene_signature = signature;
            self.scene_change_observed_at = Some(now);
        } else if self.scene_change_observed_at.is_none() {
            self.scene_change_observed_at = Some(now);
        }
    }

    pub(super) fn update_transition(&mut self, now: Instant) {
        let duration = self.transition_duration.max(0.0);
        if let Some(target) = self.transition_to {
            let complete = duration <= 0.0
                || self
                    .transition_started
                    .is_some_and(|started| now.duration_since(started).as_secs_f32() >= duration);
            if complete {
                self.front_index = target;
                self.transition_to = None;
                self.transition_started = None;
            }
        }
        if self.initial_transition_started.is_some_and(|started| {
            duration <= 0.0 || now.duration_since(started).as_secs_f32() >= duration
        }) {
            self.initial_transition_started = None;
        }
    }

    pub(super) fn transition_factor(&self, now: Instant) -> f32 {
        let Some(_) = self.transition_to else { return 0.0; };
        let duration = self.transition_duration.max(0.0);
        if duration <= 0.0 {
            return 1.0;
        }
        self.transition_started
            .map(|started| now.duration_since(started).as_secs_f32() / duration)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    }

    pub(super) fn slot_pair(&self, now: Instant) -> (u32, u32, f32) {
        if !self.has_capture {
            return (0, 0, 0.0);
        }
        if let Some(target) = self.transition_to {
            return (
                self.slots[self.front_index],
                self.slots[target],
                self.transition_factor(now),
            );
        }
        let slot = self.slots[self.front_index];
        (slot, slot, 0.0)
    }
}

#[derive(Default)]
pub(super) struct ReflectionProbeSpatialIndex {
    pub(super) cell_size: f32,
    pub(super) cells: HashMap<(i32, i32, i32), Vec<u32>>,
    /// Probes spanning too many cells are queried globally instead of
    /// exploding the grid's memory footprint.
    pub(super) oversized: Vec<u32>,
}
