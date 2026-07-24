use super::*;

// Pending-draw collection for the WGPU frame path.

impl WgpuRenderer {
    pub(super) fn prepare_pending_draws_for_frame<'a>(
        &mut self,
        frame: &'a RenderFrame,
        assets: Option<&'a RenderAssets>,
    ) -> Vec<PendingDraw<'a>> {
        self.prepare_pending_draws_for_view(
            frame,
            assets,
            frame.camera.position,
            crate::components::ALL_RENDER_LAYERS,
            None,
        )
    }

    pub(super) fn prepare_pending_draws_for_view<'a>(
        &mut self,
        frame: &'a RenderFrame,
        assets: Option<&'a RenderAssets>,
        camera_position: Vec3,
        layer_mask: u32,
        output_target_name: Option<&str>,
    ) -> Vec<PendingDraw<'a>> {
        let mut pending_draws = Vec::new();
        for object in &frame.objects {
            if object.render_layers & layer_mask == 0 {
                continue;
            }
            // A surface sampling the texture currently being rendered would
            // create an illegal read/write feedback loop. Excluding it also
            // gives mirrors and portals the expected behavior without any
            // mirror-specific component or renderer branch.
            if output_target_name.is_some_and(|target_name| {
                object
                    .custom_shader
                    .as_ref()
                    .is_some_and(|shader| shader.render_textures.iter().take(4).any(|name| name == target_name))
            }) {
                continue;
            }
            let Some(geometry) = object_geometry(object, assets, None) else { continue; };
            let sort_depth = object_sort_depth(object, camera_position);
            let pipeline = if let Some(custom) = &object.custom_shader {
                PipelineKind::Custom { key: self.ensure_custom_pipeline(custom, assets), bucket: custom.render_bucket }
            } else {
                material_pipeline_kind(&object.material)
            };
            let (bounds_min, bounds_max) = world_vertex_bounds(object, &geometry.vertices);
            if !bounds_min.x.is_finite() || !bounds_max.x.is_finite() {
                continue;
            }
            let geometry_key = geometry_cache_key(object, None);
            let geometry_signature = geometry_buffer_signature(object, &geometry, None);
            pending_draws.push(PendingDraw { object, geometry, geometry_key, geometry_signature, pipeline, use_custom_material: object.custom_shader.is_some(), sort_depth, bounds_min, bounds_max });
        }
        pending_draws
    }
}

impl WgpuRenderer {
    pub(super) fn prepare_pending_draws_for_reflection_capture<'a>(
        &mut self,
        frame: &'a RenderFrame,
        assets: Option<&'a RenderAssets>,
        camera_position: Vec3,
        probe: &RenderReflectionProbe,
    ) -> Vec<PendingDraw<'a>> {
        let layer_mask = probe.capture_include_layers & !probe.capture_exclude_layers;
        let mut pending_draws = Vec::new();
        for object in &frame.objects {
            if object.entity.0 == probe.entity.0 || object.render_layers & layer_mask == 0 {
                continue;
            }
            if matches!(object.material.alpha_mode, AlphaMode::Blend) && !probe.capture_transparent {
                continue;
            }

            let mut use_custom_material = false;
            let pipeline = if let Some(custom) = object.custom_shader.as_ref() {
                let effective_mode = match (probe.capture_custom_materials, custom.reflection_capture_mode) {
                    (crate::components::ReflectionProbeCustomMaterialCaptureMode::Exclude, _)
                    | (_, CustomShaderReflectionCaptureMode::Exclude) => {
                        continue;
                    }
                    (crate::components::ReflectionProbeCustomMaterialCaptureMode::Shader, CustomShaderReflectionCaptureMode::Shader) => {
                        use_custom_material = true;
                        PipelineKind::Custom {
                            key: self.ensure_custom_capture_pipeline(custom, assets),
                            bucket: custom.render_bucket,
                        }
                    }
                    _ => material_pipeline_kind(&object.material),
                };
                effective_mode
            } else {
                material_pipeline_kind(&object.material)
            };

            if matches!(
                &pipeline,
                PipelineKind::Custom { bucket: CustomShaderRenderBucket::Transparent, .. }
            ) && !probe.capture_transparent
            {
                continue;
            }
            if matches!(
                &pipeline,
                PipelineKind::Custom { bucket: CustomShaderRenderBucket::Overlay, .. }
            ) {
                continue;
            }

            let Some(geometry) = object_geometry(object, assets, None) else { continue; };
            let sort_depth = object_sort_depth(object, camera_position);
            let (bounds_min, bounds_max) = world_vertex_bounds(object, &geometry.vertices);
            if !bounds_min.x.is_finite() || !bounds_max.x.is_finite() {
                continue;
            }
            let geometry_key = geometry_cache_key(object, None);
            let geometry_signature = geometry_buffer_signature(object, &geometry, None);
            pending_draws.push(PendingDraw {
                object,
                geometry,
                geometry_key,
                geometry_signature,
                pipeline,
                use_custom_material,
                sort_depth,
                bounds_min,
                bounds_max,
            });
        }
        pending_draws
    }
}

