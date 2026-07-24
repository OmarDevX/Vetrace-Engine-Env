use super::*;
use std::sync::Arc;

// Split-out implementation details for `wgpu_window.rs`.

impl WgpuRenderer {
    pub(super) fn bind_scene_draw_pipeline<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, draw: &PreparedDraw) {
        match &draw.pipeline {
            PipelineKind::Default => pass.set_pipeline(&self.pipelines.default_pipeline),
            PipelineKind::DefaultDoubleSided => pass.set_pipeline(&self.pipelines.default_double_sided_pipeline),
            PipelineKind::Transparent => pass.set_pipeline(&self.pipelines.transparent_pipeline),
            PipelineKind::TransparentDoubleSided => pass.set_pipeline(&self.pipelines.transparent_double_sided_pipeline),
            PipelineKind::Custom { key, .. } => {
                let pipeline = self.pipelines.custom_pipelines.get(key).unwrap_or(&self.pipelines.default_pipeline);
                pass.set_pipeline(pipeline);
            }
            PipelineKind::OutlineMask | PipelineKind::OutlineOverlay => unreachable!("outline draws use the stencil overlay pass"),
        }
    }

    pub(super) fn prepare_shadow_draw(
        &mut self,
        pending: &PendingDraw<'_>,
        buffers: PreparedGeometryBuffers,
        candidate: ShadowCandidate,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
    ) -> PreparedShadowDraw {
        self.ensure_texture(pending.object.material.base_color_texture, assets, true);
        let mut uniform = material_uniform_from_material(&pending.object.material, None, frame);
        uniform.set_model(object_model_matrix(pending.object));
        // The shadow shader only needs alpha-mode/cutoff/base alpha. Disable
        // shadow sampling in this compact material uniform to keep the depth pass
        // independent from the texture it is currently writing.
        uniform.set_shadow(Mat4::IDENTITY, false, self.shadows.shadow_target.size as f32, 0.0, 0.0);
        let uniform_buffer = self.core.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vetrace alpha-tested shadow material uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let base_color_view = self.material_texture_view(pending.object.material.base_color_texture, true, MaterialTextureFallback::White);
        let material_bind_group = self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace alpha-tested shadow material bind group"),
            layout: &self.shadows.shadow_material_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(base_color_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.scene.texture_sampler) },
            ],
        });
        let mut vertex_count = buffers.vertex_count;
        let mut index_count = buffers.index_count;
        if buffers.index_buffer.is_some() {
            index_count = candidate.vertices.min(index_count as usize) as u32;
        } else {
            vertex_count = candidate.vertices.min(vertex_count as usize) as u32;
        }
        PreparedShadowDraw {
            vertex_buffer: buffers.vertex_buffer,
            index_buffer: buffers.index_buffer,
            material_bind_group,
            vertex_count,
            index_count,
            bounds_min: pending.bounds_min,
            bounds_max: pending.bounds_max,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_draw(
        &mut self,
        cache_key: u64,
        signature: SceneDrawSignature,
        buffers: PreparedGeometryBuffers,
        uniform: CustomShaderUniform,
        material: &Material,
        custom_shader: Option<&CustomShaderMaterial>,
        lightmap_atlas_id: Option<u64>,
        assets: Option<&RenderAssets>,
        pipeline: PipelineKind,
        sort_depth: f32,
        frame_index: u64,
    ) -> PreparedDraw {
        self.ensure_material_textures(material, assets);

        if let Some(entry) = self.scene.scene_draw_cache.get_mut(&cache_key) {
            if entry.signature == signature {
                self.core.queue.write_buffer(&entry.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
                entry.last_used_frame = frame_index;
                return PreparedDraw {
                    vertex_buffer: buffers.vertex_buffer,
                    index_buffer: buffers.index_buffer,
                    material_bind_group: entry.material_bind_group.clone(),
                    vertex_count: buffers.vertex_count,
                    index_count: buffers.index_count,
                    pipeline,
                    sort_depth,
                };
            }
        }

        let uniform_buffer = self.core.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vetrace cached material/object uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let base_color_view = self.material_texture_view(material.base_color_texture, true, MaterialTextureFallback::White);
        let normal_view = self.material_texture_view(material.normal_texture, false, MaterialTextureFallback::Normal);
        let metallic_roughness_view = self.material_texture_view(material.metallic_roughness_texture, false, MaterialTextureFallback::White);
        let occlusion_view = self.material_texture_view(material.occlusion_texture, false, MaterialTextureFallback::White);
        let emissive_view = self.material_texture_view(material.emissive_texture, true, MaterialTextureFallback::White);
        let lightmap_view = self.baked_lightmap_view(lightmap_atlas_id);
        let evsm_view = self.shadows.shadow_target.evsm_view_or(&self.shadows.dummy_evsm_moments);
        let render_texture_0 = self.custom_render_texture_view(custom_shader, 0);
        let render_texture_1 = self.custom_render_texture_view(custom_shader, 1);
        let render_texture_2 = self.custom_render_texture_view(custom_shader, 2);
        let render_texture_3 = self.custom_render_texture_view(custom_shader, 3);
        let material_bind_group = Arc::new(self.core.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vetrace cached material bind group"),
            layout: &self.scene.material_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(base_color_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.scene.texture_sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(normal_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(metallic_roughness_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(occlusion_view) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(emissive_view) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&self.shadows.shadow_target.view) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::Sampler(&self.shadows.shadow_sampler) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(evsm_view) },
                wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(lightmap_view) },
                wgpu::BindGroupEntry { binding: 11, resource: wgpu::BindingResource::TextureView(render_texture_0) },
                wgpu::BindGroupEntry { binding: 12, resource: wgpu::BindingResource::TextureView(render_texture_1) },
                wgpu::BindGroupEntry { binding: 13, resource: wgpu::BindingResource::TextureView(render_texture_2) },
                wgpu::BindGroupEntry { binding: 14, resource: wgpu::BindingResource::TextureView(render_texture_3) },
                wgpu::BindGroupEntry { binding: 15, resource: wgpu::BindingResource::Sampler(&self.scene.screen_sampler) },
            ],
        }));
        self.scene.scene_draw_cache.insert(
            cache_key,
            CachedSceneDraw {
                signature,
                uniform_buffer,
                material_bind_group: material_bind_group.clone(),
                last_used_frame: frame_index,
            },
        );
        PreparedDraw {
            vertex_buffer: buffers.vertex_buffer,
            index_buffer: buffers.index_buffer,
            material_bind_group,
            vertex_count: buffers.vertex_count,
            index_count: buffers.index_count,
            pipeline,
            sort_depth,
        }
    }

    pub(super) fn custom_render_texture_view(
        &self,
        custom_shader: Option<&CustomShaderMaterial>,
        slot: usize,
    ) -> &wgpu::TextureView {
        custom_shader
            .and_then(|shader| shader.render_textures.get(slot))
            .and_then(|name| self.scene.render_texture_targets.get(name))
            .map(|target| &target.color.view)
            .unwrap_or(&self.scene.black_linear_texture.view)
    }

    pub(super) fn evict_old_scene_draw_cache_entries(&mut self, frame_index: u64) {
        let keep_after = frame_index.saturating_sub(240);
        self.scene.scene_draw_cache.retain(|_, entry| entry.last_used_frame >= keep_after);
    }

    pub(super) fn evict_removed_texture_cache_entries(&mut self, assets: Option<&RenderAssets>) {
        let Some(assets) = assets else { return; };
        self.scene.texture_cache.retain(|(handle, _), _| assets.textures.contains_key(handle));
        self.scene.texture_cache_revisions
            .retain(|(handle, _), _| assets.textures.contains_key(handle));
    }

    pub(super) fn ensure_material_textures(&mut self, material: &Material, assets: Option<&RenderAssets>) {
        self.ensure_texture(material.base_color_texture, assets, true);
        self.ensure_texture(material.normal_texture, assets, false);
        self.ensure_texture(material.metallic_roughness_texture, assets, false);
        self.ensure_texture(material.occlusion_texture, assets, false);
        self.ensure_texture(material.emissive_texture, assets, true);
    }

    pub(in crate::wgpu_window) fn ensure_texture(&mut self, texture_handle: Option<crate::components::TextureHandle>, assets: Option<&RenderAssets>, srgb: bool) {
        let Some(texture_handle) = texture_handle else { return; };
        let key = (texture_handle.0, srgb);
        let Some(texture_asset) = assets.and_then(|assets| assets.textures.get(&texture_handle.0)) else { return; };
        let cached_revision = self.scene.texture_cache_revisions.get(&key).copied();
        if self.scene.texture_cache.contains_key(&key) && cached_revision == Some(texture_asset.revision) {
            return;
        }
        let color_space = if srgb { "srgb" } else { "linear" };
        let label = format!("vetrace {color_space} material texture: {}", texture_asset.name);
        let gpu_texture = upload_texture_asset(&self.core.device, &self.core.queue, &label, texture_asset, srgb);
        self.scene.texture_cache.insert(key, gpu_texture);
        self.scene.texture_cache_revisions.insert(key, texture_asset.revision);
        // Cached material bind groups hold the old texture view. Rebuild them
        // after an in-place asset edit so runtime texture changes are visible.
        self.scene.scene_draw_cache.clear();
    }

    pub(in crate::wgpu_window) fn sprite_texture_view(&self, texture_handle: Option<crate::components::TextureHandle>, srgb: bool) -> &wgpu::TextureView {
        self.material_texture_view(texture_handle, srgb, MaterialTextureFallback::White)
    }

    pub(super) fn material_texture_view(&self, texture_handle: Option<crate::components::TextureHandle>, srgb: bool, fallback: MaterialTextureFallback) -> &wgpu::TextureView {
        if let Some(handle) = texture_handle {
            if let Some(texture) = self.scene.texture_cache.get(&(handle.0, srgb)) {
                return &texture.view;
            }
        }
        match fallback {
            MaterialTextureFallback::White => {
                if srgb { &self.scene.white_srgb_texture.view } else { &self.scene.white_linear_texture.view }
            }
            MaterialTextureFallback::Normal => &self.scene.neutral_normal_texture.view,
        }
    }
}
