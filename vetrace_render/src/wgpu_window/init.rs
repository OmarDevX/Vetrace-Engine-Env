use super::*;
use std::sync::Arc;

// Shared renderer initialization plus the optional desktop window adapter.

impl WgpuRenderer {
    /// Builds the complete Vetrace renderer around an already-created WGPU
    /// surface. Desktop and browser adapters both use this constructor, so all
    /// render passes, caches, materials, lighting, reflections, and
    /// post-processing stay on one implementation path.
    pub async fn from_surface(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        present_mode_preference: PresentModePreference,
        adapter_preference: AdapterPreference,
    ) -> Result<Self, String> {
        let initial_device = request_initial_device(&instance, &surface, adapter_preference).await?;
        #[cfg(feature = "profiler")]
        let gpu_timestamp_profiler = initial_device.gpu_timestamp_profiler;
        let adapter = initial_device.adapter;
        let backend_label = match adapter.get_info().backend {
            wgpu::Backend::BrowserWebGpu => "WebGPU".to_string(),
            wgpu::Backend::Vulkan => "Vulkan".to_string(),
            wgpu::Backend::Metal => "Metal".to_string(),
            wgpu::Backend::Dx12 => "DirectX 12".to_string(),
            wgpu::Backend::Gl => "OpenGL / WebGL".to_string(),
            other => format!("{other:?}"),
        };
        let device = initial_device.device;
        let queue = initial_device.queue;
        let config = configure_initial_surface(
            &surface,
            &adapter,
            &device,
            width,
            height,
            present_mode_preference,
        );
        let surface_view_format = render_view_format(config.format);
        let depth = DepthTarget::new(&device, config.width, config.height);
        let shadow_target = ShadowTarget::new(&device, DEFAULT_SHADOW_MAP_SIZE, SHADOW_CASCADE_COUNT as u32);
        let layouts = create_initial_layouts(&device);
        let gpu_assets = create_initial_gpu_assets(&device, &queue);
        let environment_assets = create_initial_environment_resources(&device, &queue, &layouts.environment_layout);
        let surface_info = GpuSurfaceConfig { format: surface_view_format };
        let pipelines = create_initial_pipelines(&device, surface_info, &layouts);
        #[cfg(feature = "render_2d")]
        let canvas_2d = Canvas2DRendererState::new(&device, surface_view_format);

        #[cfg(feature = "egui_render")]
        let egui_ctx = egui::Context::default();
        #[cfg(feature = "egui_render")]
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_view_format, None, 1);

        Ok(Self {
            core: WgpuCoreState {
                instance,
                surface,
                device,
                queue,
                config,
                surface_view_format,
                depth,
                backend_label,
                pixel_scale_factor: 1.0,
            },
            shadows: ShadowRendererState {
                shadow_target,
                shadow_pipeline: pipelines.shadow_pipeline,
                evsm_moment_pipeline: pipelines.evsm_moment_pipeline,
                evsm_blur_pipeline: pipelines.evsm_blur_pipeline,
                evsm_moment_layout: layouts.evsm_moment_layout,
                evsm_blur_layout: layouts.evsm_blur_layout,
                shadow_material_layout: layouts.shadow_material_layout,
                shadow_camera_buffers: layouts.shadow_camera_buffers,
                shadow_camera_bind_groups: layouts.shadow_camera_bind_groups,
                shadow_sampler: gpu_assets.shadow_sampler,
                dummy_evsm_moments: gpu_assets.dummy_evsm_moments,
                shadow_cache_frame: 0,
            },
            environment: EnvironmentRendererState {
                environment_layout: layouts.environment_layout,
                environment_bind_group: environment_assets.bind_group,
                environment_uniform_buffer: environment_assets.uniform_buffer,
                capture_environment_bind_group: environment_assets.capture_bind_group,
                capture_environment_uniform_buffer: environment_assets.capture_uniform_buffer,
                reflection_probe_buffer: environment_assets.probe_buffer,
                reflection_prefilter_layout: environment_assets.prefilter_layout,
                reflection_prefilter_pipeline: environment_assets.prefilter_pipeline,
                _environment_brdf_lut: environment_assets.brdf_lut,
                environment_cubemap_pool: environment_assets.cubemap_pool,
                reflection_probe_selection_cache: HashMap::new(),
                reflection_probe_spatial_index: ReflectionProbeSpatialIndex::default(),
                reflection_probe_capture_states: HashMap::new(),
                reflection_faces_captured_this_frame: 0,
                reflection_mips_filtered_this_frame: 0,
                reflection_probe_evictions_total: 0,
                environment_sky_enabled: false,
                capture_sky_enabled: false,
            },
            post_process: PostProcessRendererState {
                ssao_pipeline: pipelines.ssao_pipeline,
                ssao_blur_pipeline: pipelines.ssao_blur_pipeline,
                ssao_composite_pipeline: pipelines.ssao_composite_pipeline,
                fxaa_pipeline: pipelines.fxaa_pipeline,
                post_process_copy_pipeline: pipelines.post_process_copy_pipeline,
                ssao_layout: layouts.ssao_layout,
                ssao_blur_layout: layouts.ssao_blur_layout,
                ssao_composite_layout: layouts.ssao_composite_layout,
                custom_post_process_layout: layouts.custom_post_process_layout,
                ao_target: None,
                ssao_uniform_buffer: gpu_assets.ssao_uniform_buffer,
                custom_post_process_uniform_buffer: gpu_assets.custom_post_process_uniform_buffer,
                custom_post_process_uniform_buffers: Vec::new(),
                post_process_target_a: None,
                post_process_target_b: None,
                ssr_history: None,
                ssr_history_valid: false,
                previous_post_process_view_proj: Mat4::IDENTITY,
                custom_post_process_pipelines: HashMap::new(),
            },
            scene: SceneGpuState {
                material_layout: layouts.material_layout,
                camera_layout: layouts.camera_layout,
                camera_buffer: layouts.camera_buffer,
                camera_bind_group: layouts.camera_bind_group,
                texture_sampler: gpu_assets.texture_sampler,
                screen_sampler: gpu_assets.screen_sampler,
                white_srgb_texture: gpu_assets.white_srgb_texture,
                white_linear_texture: gpu_assets.white_linear_texture,
                black_linear_texture: gpu_assets.black_linear_texture,
                neutral_normal_texture: gpu_assets.neutral_normal_texture,
                render_texture_targets: HashMap::new(),
                texture_cache: HashMap::new(),
                texture_cache_revisions: HashMap::new(),
                baked_lightmap_texture: None,
                geometry_buffer_cache: HashMap::new(),
                scene_draw_cache: HashMap::new(),
                scene_cache_frame: 0,
            },
            pipelines: RenderPipelineState {
                default_pipeline: pipelines.default_pipeline,
                default_double_sided_pipeline: pipelines.default_double_sided_pipeline,
                transparent_pipeline: pipelines.transparent_pipeline,
                transparent_double_sided_pipeline: pipelines.transparent_double_sided_pipeline,
                sky_pipeline: pipelines.sky_pipeline,
                capture_default_pipeline: pipelines.capture_default_pipeline,
                capture_default_double_sided_pipeline: pipelines.capture_default_double_sided_pipeline,
                capture_transparent_pipeline: pipelines.capture_transparent_pipeline,
                capture_transparent_double_sided_pipeline: pipelines.capture_transparent_double_sided_pipeline,
                capture_sky_pipeline: pipelines.capture_sky_pipeline,
                outline_mask_pipeline: pipelines.outline_mask_pipeline,
                outline_overlay_pipeline: pipelines.outline_overlay_pipeline,
                overlay_pipeline: pipelines.overlay_pipeline,
                custom_modules: HashMap::new(),
                custom_pipelines: HashMap::new(),
                custom_capture_pipelines: HashMap::new(),
            },
            optional: OptionalRendererState {
                #[cfg(feature = "profiler")]
                gpu_timestamp_profiler,
                #[cfg(feature = "egui_render")]
                egui_ctx,
                #[cfg(feature = "egui_render")]
                egui_renderer,
                #[cfg(all(feature = "egui_render", feature = "profiler"))]
                profiler_sort_mode: 0,
                #[cfg(all(feature = "egui_render", feature = "profiler"))]
                profiler_include_overhead: false,
                #[cfg(all(feature = "egui_render", feature = "profiler", feature = "wgpu_window"))]
                detached_profiler: None,
                #[cfg(all(feature = "egui_render", feature = "profiler", feature = "wgpu_window"))]
                detached_profiler_closed_by_user: false,
            },
            #[cfg(feature = "render_2d")]
            canvas_2d,
            #[cfg(feature = "wgpu_window")]
            desktop: None,
        })
    }

    /// Surface dimensions currently used for camera projection and render
    /// targets. Browser adapters use this to synchronize `RenderSettings`.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.core.config.width, self.core.config.height)
    }

    /// Human-readable WGPU backend selected by the adapter.
    pub fn backend_label(&self) -> &str {
        &self.core.backend_label
    }

    /// Sets the physical-pixels-per-logical-point scale used by runtime UI.
    /// Desktop adapters derive this from the native window and browser adapters
    /// derive it from `devicePixelRatio`.
    pub fn set_pixel_scale_factor(&mut self, scale: f32) {
        self.core.pixel_scale_factor = if scale.is_finite() { scale.max(0.001) } else { 1.0 };
    }

    #[cfg(feature = "wgpu_window")]
    /// Construct a desktop WGPU target while honoring the complete
    /// window/cursor policy supplied by `RenderSettings`.
    pub fn new_from_render_settings(settings: RenderSettings) -> Result<Self, String> {
        let RenderSettings {
            title,
            width,
            height,
            present_mode,
            adapter_preference,
            cursor_grab,
            cursor_visible,
            ..
        } = settings;
        pollster::block_on(Self::new_desktop_async(
            title,
            width,
            height,
            present_mode,
            adapter_preference,
            cursor_grab,
            cursor_visible,
        ))
    }

    #[cfg(feature = "wgpu_window")]
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Result<Self, String> {
        Self::new_with_settings(title, width, height, PresentModePreference::default(), AdapterPreference::default())
    }

    #[cfg(feature = "wgpu_window")]
    pub fn new_with_present_mode(
        title: impl Into<String>,
        width: u32,
        height: u32,
        present_mode_preference: PresentModePreference,
    ) -> Result<Self, String> {
        Self::new_with_settings(title, width, height, present_mode_preference, AdapterPreference::default())
    }

    #[cfg(feature = "wgpu_window")]
    pub fn new_with_settings(
        title: impl Into<String>,
        width: u32,
        height: u32,
        present_mode_preference: PresentModePreference,
        adapter_preference: AdapterPreference,
    ) -> Result<Self, String> {
        pollster::block_on(Self::new_desktop_async(
            title.into(),
            width,
            height,
            present_mode_preference,
            adapter_preference,
            true,
            false,
        ))
    }

    #[cfg(feature = "wgpu_window")]
    async fn new_desktop_async(
        title: String,
        width: u32,
        height: u32,
        present_mode_preference: PresentModePreference,
        adapter_preference: AdapterPreference,
        cursor_grab: bool,
        cursor_visible: bool,
    ) -> Result<Self, String> {
        let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
        let window = Arc::new(
            WindowBuilder::new()
                .with_title(title)
                .with_inner_size(PhysicalSize::new(width.max(1), height.max(1)))
                .build(&event_loop)
                .map_err(|err| err.to_string())?,
        );
        window.set_cursor_visible(cursor_visible || !cursor_grab);
        if cursor_grab {
            let _ = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
        }

        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|err| format!("failed to create WGPU surface: {err}"))?;
        let mut renderer = Self::from_surface(
            instance,
            surface,
            width,
            height,
            present_mode_preference,
            adapter_preference,
        )
        .await?;
        renderer.set_pixel_scale_factor(window.scale_factor() as f32);
        renderer.desktop = Some(DesktopWindowState {
            event_loop,
            window,
            game_window_focused: true,
        });
        Ok(renderer)
    }
}
