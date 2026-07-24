use super::*;

// Default WGPU render pipelines created during window initialization.

pub(super) struct InitialPipelines {
    pub(super) default_pipeline: wgpu::RenderPipeline,
    pub(super) default_double_sided_pipeline: wgpu::RenderPipeline,
    pub(super) transparent_pipeline: wgpu::RenderPipeline,
    pub(super) transparent_double_sided_pipeline: wgpu::RenderPipeline,
    pub(super) sky_pipeline: wgpu::RenderPipeline,
    pub(super) capture_default_pipeline: wgpu::RenderPipeline,
    pub(super) capture_default_double_sided_pipeline: wgpu::RenderPipeline,
    pub(super) capture_transparent_pipeline: wgpu::RenderPipeline,
    pub(super) capture_transparent_double_sided_pipeline: wgpu::RenderPipeline,
    pub(super) capture_sky_pipeline: wgpu::RenderPipeline,
    pub(super) shadow_pipeline: wgpu::RenderPipeline,
    pub(super) evsm_moment_pipeline: wgpu::RenderPipeline,
    pub(super) evsm_blur_pipeline: wgpu::RenderPipeline,
    pub(super) ssao_pipeline: wgpu::RenderPipeline,
    pub(super) ssao_blur_pipeline: wgpu::RenderPipeline,
    pub(super) ssao_composite_pipeline: wgpu::RenderPipeline,
    pub(super) fxaa_pipeline: wgpu::RenderPipeline,
    pub(super) post_process_copy_pipeline: wgpu::RenderPipeline,
    pub(super) outline_mask_pipeline: wgpu::RenderPipeline,
    pub(super) outline_overlay_pipeline: wgpu::RenderPipeline,
    pub(super) overlay_pipeline: wgpu::RenderPipeline,
}

pub(super) fn create_initial_pipelines(
    device: &wgpu::Device,
    surface_info: GpuSurfaceConfig,
    layouts: &InitialLayouts,
) -> InitialPipelines {
    let default_pipeline = create_object_pipeline(
        device,
        surface_info,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace default object pipeline",
        true,
        wgpu::CompareFunction::LessEqual,
        Some(wgpu::Face::Back),
    );
    let default_double_sided_pipeline = create_object_pipeline(
        device,
        surface_info,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace default double-sided object pipeline",
        true,
        wgpu::CompareFunction::LessEqual,
        None,
    );
    let transparent_pipeline = create_object_pipeline(
        device,
        surface_info,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace transparent object pipeline",
        false,
        wgpu::CompareFunction::LessEqual,
        Some(wgpu::Face::Back),
    );
    let transparent_double_sided_pipeline = create_object_pipeline(
        device,
        surface_info,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace transparent double-sided object pipeline",
        false,
        wgpu::CompareFunction::LessEqual,
        None,
    );
    let capture_surface = GpuSurfaceConfig { format: ENVIRONMENT_TEXTURE_FORMAT };
    let capture_default_pipeline = create_object_pipeline(
        device,
        capture_surface,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace reflection capture object pipeline",
        true,
        wgpu::CompareFunction::LessEqual,
        Some(wgpu::Face::Back),
    );
    let capture_default_double_sided_pipeline = create_object_pipeline(
        device,
        capture_surface,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace reflection capture double-sided object pipeline",
        true,
        wgpu::CompareFunction::LessEqual,
        None,
    );
    let capture_transparent_pipeline = create_object_pipeline(
        device,
        capture_surface,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace reflection capture transparent object pipeline",
        false,
        wgpu::CompareFunction::LessEqual,
        Some(wgpu::Face::Back),
    );
    let capture_transparent_double_sided_pipeline = create_object_pipeline(
        device,
        capture_surface,
        &layouts.material_layout,
        &layouts.camera_layout,
        &layouts.environment_layout,
        DEFAULT_FRAGMENT_WGSL,
        "vetrace reflection capture transparent double-sided object pipeline",
        false,
        wgpu::CompareFunction::LessEqual,
        None,
    );
    let capture_sky_pipeline = create_sky_pipeline(
        device,
        capture_surface,
        &layouts.camera_layout,
        &layouts.environment_layout,
    );

    InitialPipelines {
        default_pipeline,
        default_double_sided_pipeline,
        transparent_pipeline,
        transparent_double_sided_pipeline,
        sky_pipeline: create_sky_pipeline(device, surface_info, &layouts.camera_layout, &layouts.environment_layout),
        capture_default_pipeline,
        capture_default_double_sided_pipeline,
        capture_transparent_pipeline,
        capture_transparent_double_sided_pipeline,
        capture_sky_pipeline,
        shadow_pipeline: create_shadow_pipeline(device, &layouts.shadow_material_layout, &layouts.camera_layout),
        evsm_moment_pipeline: create_evsm_moment_pipeline(device, &layouts.evsm_moment_layout),
        evsm_blur_pipeline: create_evsm_blur_pipeline(device, &layouts.evsm_blur_layout),
        ssao_pipeline: create_ssao_pipeline(device, &layouts.ssao_layout),
        ssao_blur_pipeline: create_ssao_blur_pipeline(device, &layouts.ssao_blur_layout),
        ssao_composite_pipeline: create_ssao_composite_pipeline(device, &layouts.ssao_composite_layout, surface_info),
        fxaa_pipeline: create_custom_post_process_pipeline(
            device,
            &layouts.custom_post_process_layout,
            FXAA_WGSL,
            "vetrace FXAA pipeline",
            surface_info.format,
        ),
        post_process_copy_pipeline: create_custom_post_process_pipeline(
            device,
            &layouts.custom_post_process_layout,
            DEFAULT_CUSTOM_POST_PROCESS_WGSL,
            "vetrace post-process copy pipeline",
            surface_info.format,
        ),
        outline_mask_pipeline: create_outline_mask_pipeline(
            device,
            surface_info,
            &layouts.material_layout,
            &layouts.camera_layout,
        ),
        outline_overlay_pipeline: create_outline_overlay_pipeline(
            device,
            surface_info,
            &layouts.material_layout,
            &layouts.camera_layout,
        ),
        overlay_pipeline: create_overlay_pipeline(device, surface_info),
    }
}
