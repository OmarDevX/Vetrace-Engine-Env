use super::*;
use std::sync::Arc;

// Geometry, shadow, and EVSM cache maintenance.

impl WgpuRenderer {
    pub(super) fn geometry_buffers_for(
        &mut self,
        key: u64,
        signature: GeometryBufferSignature,
        geometry: &IndexedGeometry,
        frame_index: u64,
    ) -> PreparedGeometryBuffers {
        if let Some(entry) = self.scene.geometry_buffer_cache.get_mut(&key) {
            if entry.signature == signature {
                entry.last_used_frame = frame_index;
                return PreparedGeometryBuffers {
                    vertex_buffer: entry.vertex_buffer.clone(),
                    index_buffer: entry.index_buffer.clone(),
                    vertex_count: entry.vertex_count,
                    index_count: entry.index_count,
                };
            }
        }

        let vertex_buffer = Arc::new(self.core.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vetrace cached indexed geometry vertices"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        let index_buffer = geometry.indices.as_ref().filter(|indices| !indices.is_empty()).map(|indices| {
            Arc::new(self.core.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vetrace cached indexed geometry indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }))
        });
        let prepared = PreparedGeometryBuffers {
            vertex_buffer: vertex_buffer.clone(),
            index_buffer: index_buffer.clone(),
            vertex_count: geometry.vertices.len() as u32,
            index_count: geometry.indices.as_ref().map(|indices| indices.len() as u32).unwrap_or(0),
        };
        self.scene.geometry_buffer_cache.insert(
            key,
            CachedGeometryBuffers {
                signature,
                vertex_buffer,
                index_buffer,
                vertex_count: prepared.vertex_count,
                index_count: prepared.index_count,
                last_used_frame: frame_index,
            },
        );
        prepared
    }

    pub(super) fn evict_old_geometry_cache_entries(&mut self, frame_index: u64) {
        // Mesh/primitive geometry is stable, so keep it longer than per-frame
        // draw state but still evict despawned/generated geometry eventually.
        let keep_after = frame_index.saturating_sub(600);
        self.scene.geometry_buffer_cache.retain(|_, entry| entry.last_used_frame >= keep_after);
    }

    pub(super) fn evict_old_shadow_cache_entries(&mut self, frame_index: u64) {
        // Shadow vertex data now shares the indexed geometry cache. This method
        // remains as a hook for shadow-specific caches/bind groups.
        let _ = frame_index;
    }

    pub(super) fn evsm_pass_uniform(direction: [f32; 2], radius_texels: f32, layer: usize, exponent: f32, shadow_map_size: u32) -> EvsmPassUniform {
        // Rgba16Float is widely renderable without optional WGPU features, but it
        // cannot hold exp(12). Clamp the EVSM warp exponent to a half-float-safe
        // range; larger values would overflow and turn the blurred moments into
        // white/light-leaking garbage on many GPUs.
        let safe_exponent = exponent.clamp(1.0, 5.5);
        EvsmPassUniform {
            direction_radius_layer: [direction[0], direction[1], radius_texels.max(0.0), layer as f32],
            exponent_size: [safe_exponent, shadow_map_size.max(1) as f32, 0.0, 0.0],
        }
    }

    pub(super) fn evsm_uniform_buffer(&self, uniform: EvsmPassUniform) -> wgpu::Buffer {
        self.core.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vetrace EVSM pass uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }
}
