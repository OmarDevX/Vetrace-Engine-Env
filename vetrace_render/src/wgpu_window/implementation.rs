use super::*;

// Keep this file focused on the public WGPU render target shape. The implementation
// lives in real child modules so each file has its own imports and compiler boundary.
#[path = "environment/resources/initialization.rs"]
mod environment_resources_initialization;
#[allow(unused_imports)]
use environment_resources_initialization::*;
#[path = "environment/resources/runtime.rs"]
mod environment_resources_runtime;
#[allow(unused_imports)]
use environment_resources_runtime::*;
#[path = "environment/resources/cubemap_upload.rs"]
mod environment_resources_cubemap_upload;
#[allow(unused_imports)]
use environment_resources_cubemap_upload::*;
#[path = "environment/resources/brdf_lut.rs"]
mod environment_resources_brdf_lut;
#[allow(unused_imports)]
use environment_resources_brdf_lut::*;
#[path = "environment/spatial_index.rs"]
mod environment_spatial_index;
#[allow(unused_imports)]
use environment_spatial_index::*;
#[path = "environment/probe_selection.rs"]
mod environment_probe_selection;
#[allow(unused_imports)]
use environment_probe_selection::*;
#[path = "environment/pipeline_builders.rs"]
mod environment_pipeline_builders;
#[allow(unused_imports)]
use environment_pipeline_builders::*;
#[path = "environment/capture/target.rs"]
mod environment_capture_target;
#[allow(unused_imports)]
use environment_capture_target::*;
#[path = "environment/capture/state.rs"]
mod environment_capture_state;
#[allow(unused_imports)]
use environment_capture_state::*;
#[path = "environment/capture/scheduling.rs"]
mod environment_capture_scheduling;
#[allow(unused_imports)]
use environment_capture_scheduling::*;
#[path = "environment/capture/shadow_passes.rs"]
mod environment_capture_shadow_passes;
#[allow(unused_imports)]
use environment_capture_shadow_passes::*;
#[path = "environment/capture/face_render.rs"]
mod environment_capture_face_render;
#[allow(unused_imports)]
use environment_capture_face_render::*;
#[path = "environment/capture/prefilter.rs"]
mod environment_capture_prefilter;
#[allow(unused_imports)]
use environment_capture_prefilter::*;
#[path = "environment/capture/helpers.rs"]
mod environment_capture_helpers;
#[allow(unused_imports)]
use environment_capture_helpers::*;
#[path = "environment/shaders.rs"]
mod environment_shaders;
#[allow(unused_imports)]
use environment_shaders::*;
#[path = "init_device.rs"]
mod init_device;
#[allow(unused_imports)]
use init_device::*;
#[path = "init_surface.rs"]
mod init_surface;
#[allow(unused_imports)]
use init_surface::*;
#[path = "init_layouts.rs"]
mod init_layouts;
#[allow(unused_imports)]
use init_layouts::*;
#[path = "init_assets.rs"]
mod init_assets;
#[allow(unused_imports)]
use init_assets::*;
#[path = "init_pipelines.rs"]
mod init_pipelines;
#[allow(unused_imports)]
use init_pipelines::*;
#[path = "init.rs"]
mod init;
#[allow(unused_imports)]
use init::*;
#[path = "lifecycle.rs"]
mod lifecycle;
#[allow(unused_imports)]
use lifecycle::*;
#[path = "cache_maintenance.rs"]
mod cache_maintenance;
#[allow(unused_imports)]
use cache_maintenance::*;
#[path = "gpu_memory_reporting.rs"]
mod gpu_memory_reporting;
#[allow(unused_imports)]
use gpu_memory_reporting::*;
#[path = "evsm_passes.rs"]
mod evsm_passes;
#[allow(unused_imports)]
use evsm_passes::*;
#[path = "ambient_occlusion.rs"]
mod ambient_occlusion;
#[allow(unused_imports)]
use ambient_occlusion::*;
#[path = "custom_post_process/runtime.rs"]
mod custom_post_process_runtime;
#[allow(unused_imports)]
use custom_post_process_runtime::*;
#[path = "custom_post_process/pipeline.rs"]
mod custom_post_process_pipeline;
#[allow(unused_imports)]
use custom_post_process_pipeline::*;
#[path = "pipeline_reload.rs"]
mod pipeline_reload;
#[allow(unused_imports)]
use pipeline_reload::*;
#[path = "overlay_geometry.rs"]
mod overlay_geometry;
#[allow(unused_imports)]
use overlay_geometry::*;
#[path = "screen_overlay_pass.rs"]
mod screen_overlay_pass;
#[allow(unused_imports)]
use screen_overlay_pass::*;
#[path = "frame_pending_draws.rs"]
mod frame_pending_draws;
#[allow(unused_imports)]
use frame_pending_draws::*;
#[path = "frame_shadow_pass.rs"]
mod frame_shadow_pass;
#[allow(unused_imports)]
use frame_shadow_pass::*;
#[path = "frame_scene_prepare.rs"]
mod frame_scene_prepare;
#[allow(unused_imports)]
use frame_scene_prepare::*;
#[path = "frame_scene_pass.rs"]
mod frame_scene_pass;
#[allow(unused_imports)]
use frame_scene_pass::*;
#[path = "render_texture_views.rs"]
mod render_texture_views;
#[allow(unused_imports)]
use render_texture_views::*;
#[path = "frame_render.rs"]
mod frame_render;
#[allow(unused_imports)]
use frame_render::*;
#[path = "frame_lifecycle.rs"]
mod frame_lifecycle;
#[allow(unused_imports)]
use frame_lifecycle::*;
#[cfg(feature = "wgpu_window")]
#[path = "render_target_impl.rs"]
mod render_target_impl;
#[cfg(feature = "wgpu_window")]
#[allow(unused_imports)]
use render_target_impl::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/core.rs"]
mod egui_overlay_core;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_core::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/screen_ui.rs"]
mod egui_overlay_screen_ui;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_screen_ui::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/world_ui.rs"]
mod egui_overlay_world_ui;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_world_ui::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/helpers.rs"]
mod egui_overlay_helpers;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_helpers::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/panel.rs"]
mod egui_overlay_panel;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_panel::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/profiler_window.rs"]
mod egui_overlay_profiler_window;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_profiler_window::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/profiler_classification.rs"]
mod egui_overlay_profiler_classification;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_profiler_classification::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/profiler_tables.rs"]
mod egui_overlay_profiler_tables;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_profiler_tables::*;
#[cfg(feature = "egui_render")]
#[path = "egui_overlay/profiler_format.rs"]
mod egui_overlay_profiler_format;
#[cfg(feature = "egui_render")]
#[allow(unused_imports)]
use egui_overlay_profiler_format::*;
#[cfg(all(feature = "egui_render", feature = "wgpu_window"))]
#[path = "egui_overlay/detached_controller.rs"]
mod egui_overlay_detached_controller;
#[cfg(all(feature = "egui_render", feature = "wgpu_window"))]
#[allow(unused_imports)]
use egui_overlay_detached_controller::*;
#[cfg(all(feature = "egui_render", feature = "wgpu_window"))]
#[path = "egui_overlay/detached_window.rs"]
mod egui_overlay_detached_window;
#[cfg(all(feature = "egui_render", feature = "wgpu_window"))]
#[allow(unused_imports)]
use egui_overlay_detached_window::*;
#[path = "draw_cache.rs"]
mod draw_cache;
#[allow(unused_imports)]
use draw_cache::*;
#[path = "texture_resources.rs"]
mod texture_resources;
#[allow(unused_imports)]
use texture_resources::*;
#[path = "evsm_pipeline_builders.rs"]
mod evsm_pipeline_builders;
#[allow(unused_imports)]
use evsm_pipeline_builders::*;
#[path = "ambient_occlusion_pipeline_builders.rs"]
mod ambient_occlusion_pipeline_builders;
#[allow(unused_imports)]
use ambient_occlusion_pipeline_builders::*;
#[path = "object_pipeline_builders.rs"]
mod object_pipeline_builders;
#[allow(unused_imports)]
use object_pipeline_builders::*;
#[path = "outline_pipeline_builders.rs"]
mod outline_pipeline_builders;
#[allow(unused_imports)]
use outline_pipeline_builders::*;
#[path = "shadow_overlay_pipeline_builders.rs"]
mod shadow_overlay_pipeline_builders;
#[allow(unused_imports)]
use shadow_overlay_pipeline_builders::*;
#[path = "shadow_selection.rs"]
mod shadow_selection;
#[allow(unused_imports)]
use shadow_selection::*;
#[path = "shadow_info.rs"]
mod shadow_info;
#[allow(unused_imports)]
use shadow_info::*;
#[path = "csm.rs"]
mod csm;
#[allow(unused_imports)]
use csm::*;
#[path = "frame_uniforms.rs"]
mod frame_uniforms;
#[allow(unused_imports)]
use frame_uniforms::*;
#[path = "geometry.rs"]
mod geometry;
#[allow(unused_imports)]
use geometry::*;
#[cfg(feature = "wgpu_window")]
#[path = "input.rs"]
mod input;
#[cfg(feature = "wgpu_window")]
#[allow(unused_imports)]
use input::*;
#[path = "shadow_shader.rs"]
mod shadow_shader;
#[allow(unused_imports)]
use shadow_shader::*;
#[path = "object_vertex_shaders.rs"]
mod object_vertex_shaders;
#[allow(unused_imports)]
use object_vertex_shaders::*;
#[path = "evsm_shaders.rs"]
mod evsm_shaders;
#[allow(unused_imports)]
use evsm_shaders::*;
#[path = "ambient_occlusion_shaders.rs"]
mod ambient_occlusion_shaders;
#[allow(unused_imports)]
use ambient_occlusion_shaders::*;
#[path = "default_fragment_shader.rs"]
mod default_fragment_shader;
#[allow(unused_imports)]
use default_fragment_shader::*;
#[path = "outline_overlay_shaders.rs"]
mod outline_overlay_shaders;
#[allow(unused_imports)]
use outline_overlay_shaders::*;

#[cfg(test)]
mod active_inline_shader_validation_tests {
    use super::*;

    fn validate_shader(label: &str, source: &str) {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("WGSL parse failed for {label}: {error}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("WGSL semantic validation failed for {label}: {error}"));
    }

    #[test]
    fn active_inline_wgpu_shaders_validate() {
        for (label, source) in [
            ("default material fragment", DEFAULT_FRAGMENT_WGSL),
            ("object vertex full", OBJECT_VERTEX_WGSL),
            ("object vertex textured", TEXTURED_OBJECT_VERTEX_WGSL),
            ("object vertex legacy", LEGACY_OBJECT_VERTEX_WGSL),
            ("directional shadow", SHADOW_WGSL),
            ("EVSM moments", EVSM_MOMENT_WGSL),
            ("EVSM blur", EVSM_BLUR_WGSL),
            ("SSAO", SSAO_WGSL),
            ("SSAO blur", SSAO_BLUR_WGSL),
            ("SSAO composite", SSAO_COMPOSITE_WGSL),
            ("sky", SKY_WGSL),
            ("reflection prefilter", REFLECTION_PREFILTER_WGSL),
            ("FXAA", FXAA_WGSL),
            ("post-process copy", DEFAULT_CUSTOM_POST_PROCESS_WGSL),
            ("outline mask", OUTLINE_MASK_FRAGMENT_WGSL),
            ("outline overlay", OUTLINE_FRAGMENT_WGSL),
            ("screen overlay", OVERLAY_WGSL),
        ] {
            validate_shader(label, source);
        }
    }
}
