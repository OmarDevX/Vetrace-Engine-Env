use super::*;

// Scene draw preparation for the WGPU frame path.

pub(super) struct PreparedSceneDraws {
    pub(super) opaque_draws: Vec<PreparedDraw>,
    pub(super) transparent_draws: Vec<PreparedDraw>,
    pub(super) overlay_draws: Vec<PreparedDraw>,
    pub(super) outline_draws: Vec<PreparedOutlineDraw>,
}

impl WgpuRenderer {
    pub(super) fn prepare_scene_draws_for_frame(
        &mut self,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        pending_draws: Vec<PendingDraw<'_>>,
        shadow_info: &ShadowInfo,
        scene_frame: u64,
    ) -> PreparedSceneDraws {
        let mut opaque_draws = Vec::new();
        let mut transparent_draws = Vec::new();
        let mut overlay_draws = Vec::new();
        let mut outline_draws = Vec::new();
        for pending in pending_draws {
            let custom_material = if pending.use_custom_material { pending.object.custom_shader.as_ref() } else { None };
            let mut uniform = material_uniform_from_material(&pending.object.material, custom_material, frame);
            uniform.set_shadow_cascades(
                shadow_info.view_proj,
                shadow_info.cascade_splits,
                shadow_info.cascade_count,
                shadow_info.enabled,
                self.shadows.shadow_target.size as f32,
                shadow_info.bias,
                shadow_info.soft_radius,
                shadow_info.pcf_quality,
                shadow_info.filter_mode.shader_value(),
                shadow_info.pcss_light_radius,
                shadow_info.slope_bias,
                shadow_info.normal_bias,
                shadow_info.evsm_blur_radius,
                shadow_info.evsm_exponent,
            );
            uniform.set_model(object_model_matrix(pending.object));

            let indirect_enabled = !matches!(
                frame.post_processing.gi_mode,
                crate::components::GlobalIlluminationMode::Off
            );
            let baked_lightmap = if indirect_enabled {
                pending.object.baked_lightmap.as_ref()
            } else {
                None
            };
            let lightmap_transform = baked_lightmap.map(|lightmap| lightmap.region.uv_scale_offset);
            let lightmap_intensity = baked_lightmap
                .map(|lightmap| lightmap.region.intensity)
                .unwrap_or(0.0);
            let baked_probes = if indirect_enabled {
                pending.object.baked_probes
            } else {
                None
            };
            let probes = baked_probes.map(|probes| (probes.sample, probes.intensity));
            let debug_mode = baked_lightmap
                .map(|lightmap| lightmap.debug_mode)
                .or_else(|| baked_probes.map(|probes| probes.debug_mode))
                .unwrap_or_default();
            let runtime_mode = baked_lightmap
                .map(|lightmap| lightmap.runtime_mode)
                .unwrap_or_default();
            let static_lighting_only = baked_lightmap
                .map(|lightmap| lightmap.region.static_lighting_only)
                .unwrap_or(false);
            let preserve_local_lights = baked_lightmap
                .map(|lightmap| lightmap.region.preserve_local_lights)
                .unwrap_or(true);
            uniform.set_baked_lighting(
                lightmap_transform,
                lightmap_intensity,
                probes,
                debug_mode,
                runtime_mode,
                static_lighting_only,
                preserve_local_lights,
            );
            let (reflection_probe_indices, reflection_probe_count) =
                self.selected_reflection_probe_indices(frame, &pending, scene_frame);
            uniform.set_reflection_probes(reflection_probe_indices, reflection_probe_count);
            let buffers = self.geometry_buffers_for(pending.geometry_key, pending.geometry_signature, &pending.geometry, scene_frame);
            let signature = scene_draw_signature(&pending, 0);
            let draw = self.prepare_draw(
                scene_cache_key(pending.object.entity.0, 0),
                signature,
                buffers,
                uniform,
                &pending.object.material,
                custom_material,
                baked_lightmap.map(|lightmap| lightmap.atlas.id),
                assets,
                pending.pipeline.clone(),
                pending.sort_depth,
                scene_frame,
            );
            if is_overlay_pipeline(&draw.pipeline) {
                overlay_draws.push(draw);
            } else if is_transparent_pipeline(&draw.pipeline) {
                transparent_draws.push(draw);
            } else {
                opaque_draws.push(draw);
            }

            // Stylized through-depth outline.  The old ground-clamp fix deformed
            // the expanded hull and made the silhouette uneven.  Instead render
            // outlines as an overlay that ignores scene depth, but first mark the
            // object's own projected interior in stencil so the outline cannot
            // become a filled dark shell over the custom material/gradient.
            if let Some(outline) = &pending.object.outline {
                if outline.enabled {
                    if let Some(mask_geometry) = object_geometry(pending.object, assets, None) {
                        if let Some(outline_geometry) = object_geometry(pending.object, assets, Some(outline.thickness.max(0.02))) {
                            let mask_material = Material { base_color: Vec3::ZERO, emissive: Vec3::ZERO, alpha: 0.0, ..Material::default() };
                            let outline_material = Material { base_color: outline.color, emissive: Vec3::ZERO, alpha: 1.0, ..Material::default() };
                            let mut mask_uniform = material_uniform_from_material(&mask_material, None, frame);
                            mask_uniform.set_shadow(Mat4::IDENTITY, false, self.shadows.shadow_target.size as f32, shadow_info.bias, 0.0);
                            let mut outline_uniform = material_uniform_from_material(&outline_material, None, frame);
                            outline_uniform.set_shadow(Mat4::IDENTITY, false, self.shadows.shadow_target.size as f32, shadow_info.bias, 0.0);
                            mask_uniform.set_model(object_model_matrix(pending.object));
                            outline_uniform.set_model(object_model_matrix(pending.object));
                            let mask_buffers = self.geometry_buffers_for(geometry_cache_key(pending.object, None), geometry_buffer_signature(pending.object, &mask_geometry, None), &mask_geometry, scene_frame);
                            let outline_extra = outline.thickness.max(0.02);
                            let outline_buffers = self.geometry_buffers_for(geometry_cache_key(pending.object, Some(outline_extra)), geometry_buffer_signature(pending.object, &outline_geometry, Some(outline_extra)), &outline_geometry, scene_frame);
                            let mask_signature = outline_scene_draw_signature(
                                pending.object,
                                &mask_material,
                                &PipelineKind::OutlineMask,
                                mask_geometry.vertices.len(),
                                mask_geometry.indices.as_ref().map(|indices| indices.len()).unwrap_or(0),
                                None,
                                0,
                            );
                            let outline_signature = outline_scene_draw_signature(
                                pending.object,
                                &outline_material,
                                &PipelineKind::OutlineOverlay,
                                outline_geometry.vertices.len(),
                                outline_geometry.indices.as_ref().map(|indices| indices.len()).unwrap_or(0),
                                None,
                                outline_extra.to_bits(),
                            );
                            outline_draws.push(PreparedOutlineDraw {
                                mask: self.prepare_draw(
                                    scene_cache_key(pending.object.entity.0, 1),
                                    mask_signature,
                                    mask_buffers,
                                    mask_uniform,
                                    &mask_material,
                                    None,
                                    None,
                                    assets,
                                    PipelineKind::OutlineMask,
                                    pending.sort_depth,
                                    scene_frame,
                                ),
                                outline: self.prepare_draw(
                                    scene_cache_key(pending.object.entity.0, 2),
                                    outline_signature,
                                    outline_buffers,
                                    outline_uniform,
                                    &outline_material,
                                    None,
                                    None,
                                    assets,
                                    PipelineKind::OutlineOverlay,
                                    pending.sort_depth,
                                    scene_frame,
                                ),
                            });
                        }
                    }
                }
            }
        }

        self.prune_reflection_probe_selection_cache(scene_frame);
        PreparedSceneDraws { opaque_draws, transparent_draws, overlay_draws, outline_draws }
    }
}
