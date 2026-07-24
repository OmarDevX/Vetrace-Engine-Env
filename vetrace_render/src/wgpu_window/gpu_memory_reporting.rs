use super::*;

// Optional profiler counters and GPU memory estimates.

#[cfg(feature = "profiler")]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GpuMemoryBreakdown {
    pub(super) textures_bytes: u64,
    pub(super) buffers_bytes: u64,
    pub(super) shadow_maps_bytes: u64,
    pub(super) ssao_targets_bytes: u64,
    pub(super) mesh_buffers_bytes: u64,
    pub(super) uniform_buffers_bytes: u64,
}

#[cfg(feature = "profiler")]
impl GpuMemoryBreakdown {
    pub(super) fn total(self) -> u64 {
        self.textures_bytes
            .saturating_add(self.shadow_maps_bytes)
            .saturating_add(self.ssao_targets_bytes)
            .saturating_add(self.mesh_buffers_bytes)
            .saturating_add(self.uniform_buffers_bytes)
    }
}

#[cfg(feature = "profiler")]
impl WgpuRenderer {
    pub(super) fn record_profiler_counters(&self, frame: &RenderFrame, pending_draws: usize, shadow_candidates: usize, opaque_draws: usize, transparent_draws: usize, outline_draws: usize) {
        vetrace_profiler::record_counter("wgpu.surface_width", self.core.config.width as f64, "px");
        vetrace_profiler::record_counter("wgpu.surface_height", self.core.config.height as f64, "px");
        vetrace_profiler::record_counter("wgpu.pending_draws", pending_draws as f64, "");
        vetrace_profiler::record_counter("wgpu.shadow_candidates", shadow_candidates as f64, "");
        vetrace_profiler::record_counter("wgpu.opaque_draws", opaque_draws as f64, "");
        vetrace_profiler::record_counter("wgpu.transparent_draws", transparent_draws as f64, "");
        vetrace_profiler::record_counter("wgpu.outline_draws", outline_draws as f64, "");
        vetrace_profiler::record_counter("wgpu.texture_cache_entries", self.scene.texture_cache.len() as f64, "");
        vetrace_profiler::record_counter("wgpu.geometry_cache_entries", self.scene.geometry_buffer_cache.len() as f64, "");
        vetrace_profiler::record_counter("wgpu.scene_draw_cache_entries", self.scene.scene_draw_cache.len() as f64, "");
        vetrace_profiler::record_counter("wgpu.shadow_map_size", self.shadows.shadow_target.size as f64, "px");
        vetrace_profiler::record_counter("wgpu.shadow_cascades", self.shadows.shadow_target.layers as f64, "");
        vetrace_profiler::record_counter("wgpu.environment_cubemaps", self.environment.environment_cubemap_pool.slots.len() as f64, "");
        vetrace_profiler::record_counter("wgpu.reflection_probes", frame.reflection_probes.len().min(MAX_REFLECTION_PROBES) as f64, "");
        vetrace_profiler::record_counter("wgpu.reflection_capture_states", self.environment.reflection_probe_capture_states.len() as f64, "");
        vetrace_profiler::record_counter(
            "wgpu.reflection_captures_in_progress",
            self.environment.reflection_probe_capture_states.values().filter(|state| !matches!(state.phase, ReflectionCapturePhase::Idle)).count() as f64,
            "",
        );
        vetrace_profiler::record_counter("wgpu.reflection_faces_captured", self.environment.reflection_faces_captured_this_frame as f64, "faces/frame");
        vetrace_profiler::record_counter("wgpu.reflection_mips_filtered", self.environment.reflection_mips_filtered_this_frame as f64, "mips/frame");
        vetrace_profiler::record_counter("wgpu.reflection_probe_evictions_total", self.environment.reflection_probe_evictions_total as f64, "");
        vetrace_profiler::record_counter("wgpu.reflection_spatial_cells", self.environment.reflection_probe_spatial_index.cells.len() as f64, "");
        vetrace_profiler::record_counter("wgpu.reflection_oversized_probes", self.environment.reflection_probe_spatial_index.oversized.len() as f64, "");
        vetrace_profiler::record_counter("wgpu.ssr_history_valid", if self.post_process.ssr_history_valid { 1.0 } else { 0.0 }, "");
        vetrace_profiler::record_counter("wgpu.ssao_enabled", if Self::ao_enabled_for_frame(frame) { 1.0 } else { 0.0 }, "");
        vetrace_profiler::record_counter("wgpu.gpu.timestamp_queries_supported", if self.optional.gpu_timestamp_profiler.is_some() { 1.0 } else { 0.0 }, "");

        let memory = self.gpu_memory_breakdown();
        vetrace_profiler::record_memory_bytes("wgpu.estimated_resource_bytes", memory.total());
        vetrace_profiler::record_memory_bytes("wgpu.memory.textures_bytes", memory.textures_bytes);
        vetrace_profiler::record_memory_bytes("wgpu.memory.buffers_bytes", memory.buffers_bytes);
        vetrace_profiler::record_memory_bytes("wgpu.memory.shadow_maps_bytes", memory.shadow_maps_bytes);
        vetrace_profiler::record_memory_bytes("wgpu.memory.ssao_targets_bytes", memory.ssao_targets_bytes);
        vetrace_profiler::record_memory_bytes("wgpu.memory.mesh_buffers_bytes", memory.mesh_buffers_bytes);
        vetrace_profiler::record_memory_bytes("wgpu.memory.uniform_buffers_bytes", memory.uniform_buffers_bytes);
    }

    pub(super) fn gpu_memory_breakdown(&self) -> GpuMemoryBreakdown {
        let mut memory = GpuMemoryBreakdown::default();

        // Scene depth buffer.
        memory.textures_bytes = memory.textures_bytes.saturating_add(texture_bytes(self.core.config.width, self.core.config.height, DEPTH_FORMAT, 1));

        // Shadow depth and EVSM moment textures.
        memory.shadow_maps_bytes = memory.shadow_maps_bytes.saturating_add(texture_bytes(self.shadows.shadow_target.size, self.shadows.shadow_target.size, SHADOW_DEPTH_FORMAT, self.shadows.shadow_target.layers));
        if self.shadows.shadow_target.evsm_moments_a.is_some() {
            memory.shadow_maps_bytes = memory.shadow_maps_bytes.saturating_add(texture_bytes(self.shadows.shadow_target.size, self.shadows.shadow_target.size, EVSM_MOMENT_FORMAT, self.shadows.shadow_target.layers));
        }
        if self.shadows.shadow_target.evsm_moments_b.is_some() {
            memory.shadow_maps_bytes = memory.shadow_maps_bytes.saturating_add(texture_bytes(self.shadows.shadow_target.size, self.shadows.shadow_target.size, EVSM_MOMENT_FORMAT, self.shadows.shadow_target.layers));
        }
        memory.shadow_maps_bytes = memory.shadow_maps_bytes.saturating_add(texture_bytes(self.dummy_evsm_moments_size(), self.dummy_evsm_moments_size(), EVSM_MOMENT_FORMAT, 1));

        // SSAO scene copy, raw AO, and blurred AO targets.
        if let Some(ao) = &self.post_process.ao_target {
            memory.ssao_targets_bytes = memory.ssao_targets_bytes.saturating_add(gpu_texture_resource_bytes(&ao.scene_color));
            memory.ssao_targets_bytes = memory.ssao_targets_bytes.saturating_add(gpu_texture_resource_bytes(&ao.raw));
            memory.ssao_targets_bytes = memory.ssao_targets_bytes.saturating_add(gpu_texture_resource_bytes(&ao.blurred));
        }

        // Scene environment cube-array, including every mip and reserved slot.
        memory.textures_bytes = memory.textures_bytes.saturating_add(mipmapped_texture_bytes(
            ENVIRONMENT_CUBEMAP_FACE_SIZE,
            ENVIRONMENT_CUBEMAP_FACE_SIZE,
            ENVIRONMENT_TEXTURE_FORMAT,
            ENVIRONMENT_CUBEMAP_CAPACITY * 6,
            ENVIRONMENT_CUBEMAP_MIP_COUNT,
        ));

        // Runtime capture source cubemaps and their reusable depth targets.
        for state in self.environment.reflection_probe_capture_states.values() {
            memory.textures_bytes = memory.textures_bytes.saturating_add(texture_bytes(
                state.target.resolution,
                state.target.resolution,
                ENVIRONMENT_TEXTURE_FORMAT,
                6,
            ));
            memory.textures_bytes = memory.textures_bytes.saturating_add(texture_bytes(
                state.target.resolution,
                state.target.resolution,
                DEPTH_FORMAT,
                1,
            ));
        }
        // Shared split-sum BRDF LUT (RG16F).
        memory.textures_bytes = memory.textures_bytes.saturating_add(texture_bytes(
            128,
            128,
            wgpu::TextureFormat::Rg16Float,
            1,
        ));

        // Fallback textures and loaded texture cache.
        memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(&self.scene.white_srgb_texture));
        memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(&self.scene.white_linear_texture));
        memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(&self.scene.neutral_normal_texture));
        for texture in self.scene.texture_cache.values() {
            memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(texture));
        }
        if let Some((_, texture)) = &self.scene.baked_lightmap_texture {
            memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(texture));
        }
        if let Some(texture) = &self.post_process.post_process_target_a {
            memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(texture));
        }
        if let Some(texture) = &self.post_process.post_process_target_b {
            memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(texture));
        }
        if let Some(texture) = &self.post_process.ssr_history {
            memory.textures_bytes = memory.textures_bytes.saturating_add(gpu_texture_resource_bytes(texture));
        }

        // Mesh vertex/index buffers.
        for geometry in self.scene.geometry_buffer_cache.values() {
            memory.mesh_buffers_bytes = memory.mesh_buffers_bytes.saturating_add((geometry.vertex_count as u64).saturating_mul(std::mem::size_of::<GpuVertex>() as u64));
            memory.mesh_buffers_bytes = memory.mesh_buffers_bytes.saturating_add((geometry.index_count as u64).saturating_mul(std::mem::size_of::<u32>() as u64));
        }

        // CPU-known uniform/storage buffer sizes. WGPU can allocate extra hidden
        // driver memory; this intentionally counts engine-owned logical bytes.
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add((self.scene.scene_draw_cache.len() as u64).saturating_mul(std::mem::size_of::<CustomShaderUniform>() as u64));
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add(std::mem::size_of::<CameraUniform>() as u64);
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add((self.shadows.shadow_camera_buffers.len() as u64).saturating_mul(std::mem::size_of::<CameraUniform>() as u64));
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add(std::mem::size_of::<SsaoUniform>() as u64);
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add(
            (self.post_process.custom_post_process_uniform_buffers.len() as u64 + 1)
                .saturating_mul(std::mem::size_of::<CustomPostProcessUniform>() as u64),
        );
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add((std::mem::size_of::<EnvironmentUniform>() * 2) as u64);
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add(
            (self.environment.reflection_probe_capture_states.len() as u64)
                .saturating_mul(
                    (std::mem::size_of::<CameraUniform>()
                        * (6 + 6 * SHADOW_CASCADE_COUNT)
                        + std::mem::size_of::<ReflectionPrefilterUniform>() * 6) as u64,
                ),
        );
        memory.uniform_buffers_bytes = memory.uniform_buffers_bytes.saturating_add(
            (std::mem::size_of::<GpuReflectionProbe>() * MAX_REFLECTION_PROBES) as u64,
        );
        memory.buffers_bytes = memory.mesh_buffers_bytes.saturating_add(memory.uniform_buffers_bytes);

        memory
    }

    pub(super) fn dummy_evsm_moments_size(&self) -> u32 { 1 }
}

#[cfg(feature = "profiler")]
pub(super) fn gpu_texture_resource_bytes(texture: &GpuTextureResource) -> u64 {
    texture_bytes(texture.width, texture.height, texture.format, 1)
}

#[cfg(feature = "profiler")]
pub(super) fn texture_bytes(width: u32, height: u32, format: wgpu::TextureFormat, layers: u32) -> u64 {
    (width.max(1) as u64)
        .saturating_mul(height.max(1) as u64)
        .saturating_mul(layers.max(1) as u64)
        .saturating_mul(bytes_per_texel(format) as u64)
}

#[cfg(feature = "profiler")]
pub(super) fn mipmapped_texture_bytes(
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    layers: u32,
    mip_count: u32,
) -> u64 {
    let mut width = width.max(1);
    let mut height = height.max(1);
    let mut total = 0_u64;
    for _ in 0..mip_count.max(1) {
        total = total.saturating_add(texture_bytes(width, height, format, layers));
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    total
}

#[cfg(feature = "profiler")]
pub(super) fn bytes_per_texel(format: wgpu::TextureFormat) -> u32 {
    match format {
        wgpu::TextureFormat::R8Unorm | wgpu::TextureFormat::R8Snorm | wgpu::TextureFormat::R8Uint | wgpu::TextureFormat::R8Sint => 1,
        wgpu::TextureFormat::R16Uint
        | wgpu::TextureFormat::R16Sint
        | wgpu::TextureFormat::R16Float
        | wgpu::TextureFormat::Rg8Unorm
        | wgpu::TextureFormat::Rg8Snorm
        | wgpu::TextureFormat::Rg8Uint
        | wgpu::TextureFormat::Rg8Sint => 2,
        wgpu::TextureFormat::R32Uint
        | wgpu::TextureFormat::R32Sint
        | wgpu::TextureFormat::R32Float
        | wgpu::TextureFormat::Rg16Uint
        | wgpu::TextureFormat::Rg16Sint
        | wgpu::TextureFormat::Rg16Float
        | wgpu::TextureFormat::Rgba8Unorm
        | wgpu::TextureFormat::Rgba8UnormSrgb
        | wgpu::TextureFormat::Rgba8Snorm
        | wgpu::TextureFormat::Rgba8Uint
        | wgpu::TextureFormat::Rgba8Sint
        | wgpu::TextureFormat::Bgra8Unorm
        | wgpu::TextureFormat::Bgra8UnormSrgb
        | wgpu::TextureFormat::Depth24Plus
        | wgpu::TextureFormat::Depth24PlusStencil8
        | wgpu::TextureFormat::Depth32Float => 4,
        wgpu::TextureFormat::Rg32Uint
        | wgpu::TextureFormat::Rg32Sint
        | wgpu::TextureFormat::Rg32Float
        | wgpu::TextureFormat::Rgba16Uint
        | wgpu::TextureFormat::Rgba16Sint
        | wgpu::TextureFormat::Rgba16Float
        | wgpu::TextureFormat::Depth32FloatStencil8 => 8,
        wgpu::TextureFormat::Rgba32Uint | wgpu::TextureFormat::Rgba32Sint | wgpu::TextureFormat::Rgba32Float => 16,
        _ => 4,
    }
}
