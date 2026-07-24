use super::*;

// Split-out implementation details for `wgpu_window.rs`.

impl WgpuRenderer {
    pub(super) fn recreate_pipelines_for_surface(&mut self) {
        let info = GpuSurfaceConfig { format: self.core.surface_view_format };
        self.pipelines.default_pipeline = create_object_pipeline(
            &self.core.device,
            info,
            &self.scene.material_layout,
            &self.scene.camera_layout,
            &self.environment.environment_layout,
            DEFAULT_FRAGMENT_WGSL,
            "vetrace default object pipeline",
            true,
            wgpu::CompareFunction::LessEqual,
            Some(wgpu::Face::Back),
        );
        self.pipelines.default_double_sided_pipeline = create_object_pipeline(
            &self.core.device,
            info,
            &self.scene.material_layout,
            &self.scene.camera_layout,
            &self.environment.environment_layout,
            DEFAULT_FRAGMENT_WGSL,
            "vetrace default double-sided object pipeline",
            true,
            wgpu::CompareFunction::LessEqual,
            None,
        );
        self.pipelines.transparent_pipeline = create_object_pipeline(
            &self.core.device,
            info,
            &self.scene.material_layout,
            &self.scene.camera_layout,
            &self.environment.environment_layout,
            DEFAULT_FRAGMENT_WGSL,
            "vetrace transparent object pipeline",
            false,
            wgpu::CompareFunction::LessEqual,
            Some(wgpu::Face::Back),
        );
        self.pipelines.transparent_double_sided_pipeline = create_object_pipeline(
            &self.core.device,
            info,
            &self.scene.material_layout,
            &self.scene.camera_layout,
            &self.environment.environment_layout,
            DEFAULT_FRAGMENT_WGSL,
            "vetrace transparent double-sided object pipeline",
            false,
            wgpu::CompareFunction::LessEqual,
            None,
        );
        self.pipelines.sky_pipeline = create_sky_pipeline(&self.core.device, info, &self.scene.camera_layout, &self.environment.environment_layout);
        self.shadows.shadow_pipeline = create_shadow_pipeline(&self.core.device, &self.shadows.shadow_material_layout, &self.scene.camera_layout);
        self.shadows.evsm_moment_pipeline = create_evsm_moment_pipeline(&self.core.device, &self.shadows.evsm_moment_layout);
        self.shadows.evsm_blur_pipeline = create_evsm_blur_pipeline(&self.core.device, &self.shadows.evsm_blur_layout);
        self.post_process.ssao_pipeline = create_ssao_pipeline(&self.core.device, &self.post_process.ssao_layout);
        self.post_process.ssao_blur_pipeline = create_ssao_blur_pipeline(&self.core.device, &self.post_process.ssao_blur_layout);
        self.post_process.ssao_composite_pipeline = create_ssao_composite_pipeline(&self.core.device, &self.post_process.ssao_composite_layout, info);
        self.post_process.fxaa_pipeline = create_custom_post_process_pipeline(
            &self.core.device,
            &self.post_process.custom_post_process_layout,
            FXAA_WGSL,
            "vetrace FXAA pipeline",
            info.format,
        );
        self.post_process.post_process_copy_pipeline = create_custom_post_process_pipeline(
            &self.core.device,
            &self.post_process.custom_post_process_layout,
            DEFAULT_CUSTOM_POST_PROCESS_WGSL,
            "vetrace post-process copy pipeline",
            info.format,
        );
        self.pipelines.outline_mask_pipeline = create_outline_mask_pipeline(
            &self.core.device,
            info,
            &self.scene.material_layout,
            &self.scene.camera_layout,
        );
        self.pipelines.outline_overlay_pipeline = create_outline_overlay_pipeline(
            &self.core.device,
            info,
            &self.scene.material_layout,
            &self.scene.camera_layout,
        );
        self.pipelines.overlay_pipeline = create_overlay_pipeline(&self.core.device, info);
        self.pipelines.custom_pipelines.clear();
        self.pipelines.custom_capture_pipelines.clear();
        self.post_process.custom_post_process_pipelines.clear();
        self.post_process.ssr_history_valid = false;
        self.post_process.previous_post_process_view_proj = Mat4::IDENTITY;
    }

    pub(super) fn ensure_custom_pipeline(&mut self, material: &CustomShaderMaterial, assets: Option<&RenderAssets>) -> String {
        let shader_id = material.shader_id.clone();
        let pipeline_key = custom_pipeline_key(material);
        if !self.pipelines.custom_modules.contains_key(&shader_id) {
            let source = resolve_text_source(
                material.wgsl_source.as_deref(),
                material.asset_path.as_deref(),
                assets,
            )
            .unwrap_or_else(|| DEFAULT_FRAGMENT_WGSL.to_string());
            let module = self.core.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("vetrace custom fragment module: {shader_id}")),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
            });
            self.pipelines.custom_modules.insert(shader_id.clone(), module);
        }
        if !self.pipelines.custom_pipelines.contains_key(&pipeline_key) {
            let Some(module) = self.pipelines.custom_modules.get(&shader_id) else {
                eprintln!("WGPU: custom shader module `{shader_id}` disappeared before pipeline creation");
                return pipeline_key;
            };
            // Choose an exact inter-stage contract. WGPU 0.20 rejects both
            // missing fragment inputs and unused vertex outputs, so Textured
            // emits locations 0..2 while Full emits locations 0..5.
            let vertex = match material.vertex_interface {
                CustomShaderVertexInterface::Legacy => create_legacy_vertex_module(&self.core.device),
                CustomShaderVertexInterface::Textured => create_textured_vertex_module(&self.core.device),
                CustomShaderVertexInterface::Full => create_vertex_module(&self.core.device),
            };
            let pipeline = create_object_pipeline_from_modules(
                &self.core.device,
                GpuSurfaceConfig { format: self.core.surface_view_format },
                &self.scene.material_layout,
                &self.scene.camera_layout,
                &self.environment.environment_layout,
                &vertex,
                module,
                &format!("vetrace custom object pipeline: {pipeline_key}"),
                material.depth_write,
                custom_depth_compare(material.depth_compare),
                custom_cull_mode(material.cull_mode),
            );
            self.pipelines.custom_pipelines.insert(pipeline_key.clone(), pipeline);
        }
        pipeline_key
    }
    pub(super) fn ensure_custom_capture_pipeline(&mut self, material: &CustomShaderMaterial, assets: Option<&RenderAssets>) -> String {
        let source = resolve_text_source(
            material.reflection_capture_wgsl_source.as_deref(),
            material.reflection_capture_asset_path.as_deref(),
            assets,
        )
        .or_else(|| resolve_text_source(material.wgsl_source.as_deref(), material.asset_path.as_deref(), assets))
        .unwrap_or_else(|| DEFAULT_FRAGMENT_WGSL.to_string());
        let pipeline_key = format!(
            "capture|{}|vertex={:?}|cull={:?}|depth_write={}|depth={:?}|bucket={:?}|shader={:016x}",
            material.shader_id,
            material.vertex_interface,
            material.cull_mode,
            material.depth_write,
            material.depth_compare,
            material.render_bucket,
            stable_hash(source.as_bytes()),
        );
        if !self.pipelines.custom_capture_pipelines.contains_key(&pipeline_key) {
            let fragment = self.core.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("vetrace custom reflection capture fragment: {}", material.shader_id)),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
            });
            let vertex = match material.vertex_interface {
                CustomShaderVertexInterface::Legacy => create_legacy_vertex_module(&self.core.device),
                CustomShaderVertexInterface::Textured => create_textured_vertex_module(&self.core.device),
                CustomShaderVertexInterface::Full => create_vertex_module(&self.core.device),
            };
            let pipeline = create_object_pipeline_from_modules(
                &self.core.device,
                GpuSurfaceConfig { format: ENVIRONMENT_TEXTURE_FORMAT },
                &self.scene.material_layout,
                &self.scene.camera_layout,
                &self.environment.environment_layout,
                &vertex,
                &fragment,
                &format!("vetrace custom reflection capture pipeline: {pipeline_key}"),
                material.depth_write,
                custom_depth_compare(material.depth_compare),
                custom_cull_mode(material.cull_mode),
            );
            self.pipelines.custom_capture_pipelines.insert(pipeline_key.clone(), pipeline);
        }
        pipeline_key
    }

}


pub(super) fn resolve_text_source(
    inline_source: Option<&str>,
    asset_path: Option<&str>,
    assets: Option<&RenderAssets>,
) -> Option<String> {
    if let Some(source) = inline_source {
        return Some(source.to_string());
    }
    let path = asset_path?;
    if let Some(source) = assets.and_then(|assets| assets.text_asset(path)) {
        return Some(source.to_string());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        return fs::read_to_string(Path::new(path)).ok();
    }
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
}

pub(super) fn custom_pipeline_key(material: &CustomShaderMaterial) -> String {
    format!(
        "{}|vertex={:?}|cull={:?}|depth_write={}|depth={:?}|bucket={:?}",
        material.shader_id.as_str(),
        material.vertex_interface,
        material.cull_mode,
        material.depth_write,
        material.depth_compare,
        material.render_bucket,
    )
}

pub(super) fn custom_cull_mode(cull_mode: CustomShaderCullMode) -> Option<wgpu::Face> {
    match cull_mode {
        CustomShaderCullMode::None => None,
        CustomShaderCullMode::Front => Some(wgpu::Face::Front),
        CustomShaderCullMode::Back => Some(wgpu::Face::Back),
    }
}

pub(super) fn custom_depth_compare(depth_compare: CustomShaderDepthCompare) -> wgpu::CompareFunction {
    match depth_compare {
        CustomShaderDepthCompare::Less => wgpu::CompareFunction::Less,
        CustomShaderDepthCompare::LessEqual => wgpu::CompareFunction::LessEqual,
        CustomShaderDepthCompare::Always => wgpu::CompareFunction::Always,
    }
}
