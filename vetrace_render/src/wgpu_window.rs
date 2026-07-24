//! Shared full WGPU renderer.
//!
//! `WgpuRenderer` owns Vetrace's platform-neutral GPU implementation: geometry
//! and texture caches, materials, shadows, environment lighting, reflection
//! probes, render textures, post-processing, custom WGSL, and runtime UI.
//! Desktop and browser adapters provide different surfaces and input loops but
//! call this same renderer.
use crate::CustomShaderVertexInterface;

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(feature = "wgpu_window")]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
#[cfg(any(feature = "wgpu_window", feature = "profiler"))]
use std::time::Duration;

#[cfg(feature = "egui_render")]
use egui_wgpu::ScreenDescriptor;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec4};
use half::f16;
use vetrace_core::{Engine, InputState};
use wgpu::util::DeviceExt;
#[cfg(feature = "wgpu_window")]
use winit::dpi::PhysicalSize;
#[cfg(feature = "wgpu_window")]
use winit::event::{DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
#[cfg(feature = "wgpu_window")]
use winit::event_loop::{ControlFlow, EventLoop};
#[cfg(feature = "wgpu_window")]
use winit::keyboard::{KeyCode, PhysicalKey};
#[cfg(feature = "wgpu_window")]
use winit::platform::pump_events::EventLoopExtPumpEvents;
#[cfg(feature = "wgpu_window")]
use winit::window::{CursorGrabMode, Window, WindowBuilder, WindowId};

use crate::backend::{RenderFrame, RenderObject, RenderOverlayRect, RenderReflectionProbe, RenderTarget};
use crate::components::{
    AlphaMode, CustomShaderCullMode, CustomShaderDepthCompare, CustomShaderMaterial,
    CustomShaderReflectionCaptureMode, CustomShaderRenderBucket, Material, PrimitiveShape, ShadowMode, Shape,
};
use crate::resources::{AdapterPreference, AmbientOcclusionMode, AntiAliasingMode, Camera, CustomPostProcessPass, MeshAsset, MeshVertex, PresentModePreference, PostProcessInput, BakedLightmapAtlas, RenderAssets, RenderSettings, ShadowFilterMode, TextureAsset};
use crate::wgpu_backend::CustomShaderUniform;

// Support types are real child modules. Their internal items use `pub(super)`
// so sibling renderer modules share only explicitly declared implementation details.
#[path = "wgpu_window/gpu_vertex_types.rs"]
mod gpu_vertex_types;
#[allow(unused_imports)]
use gpu_vertex_types::*;
#[path = "wgpu_window/gpu_uniform_types.rs"]
mod gpu_uniform_types;
#[allow(unused_imports)]
use gpu_uniform_types::*;
#[path = "wgpu_window/environment/gpu_types.rs"]
mod environment_gpu_types;
#[allow(unused_imports)]
use environment_gpu_types::*;
#[path = "wgpu_window/gpu_render_targets.rs"]
mod gpu_render_targets;
#[allow(unused_imports)]
use gpu_render_targets::*;
#[path = "wgpu_window/gpu_texture_resource.rs"]
mod gpu_texture_resource;
#[allow(unused_imports)]
use gpu_texture_resource::*;
#[path = "wgpu_window/gpu_constants.rs"]
mod gpu_constants;
#[allow(unused_imports)]
use gpu_constants::*;
#[path = "wgpu_window/gpu_timestamp_profiler.rs"]
mod gpu_timestamp_profiler;
#[allow(unused_imports)]
use gpu_timestamp_profiler::*;
#[path = "wgpu_window/draw_types.rs"]
mod draw_types;
#[allow(unused_imports)]
use draw_types::*;

#[cfg(feature = "render_2d")]
#[path = "wgpu_window/canvas_2d.rs"]
mod canvas_2d;
#[cfg(feature = "render_2d")]
use canvas_2d::*;

struct WgpuCoreState {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    surface_view_format: wgpu::TextureFormat,
    depth: DepthTarget,
    backend_label: String,
    pixel_scale_factor: f32,
}

#[cfg(feature = "wgpu_window")]
struct DesktopWindowState {
    event_loop: EventLoop<()>,
    window: Arc<Window>,
    game_window_focused: bool,
}

struct ShadowRendererState {
    shadow_target: ShadowTarget,
    shadow_pipeline: wgpu::RenderPipeline,
    evsm_moment_pipeline: wgpu::RenderPipeline,
    evsm_blur_pipeline: wgpu::RenderPipeline,
    evsm_moment_layout: wgpu::BindGroupLayout,
    evsm_blur_layout: wgpu::BindGroupLayout,
    shadow_material_layout: wgpu::BindGroupLayout,
    shadow_camera_buffers: Vec<wgpu::Buffer>,
    shadow_camera_bind_groups: Vec<wgpu::BindGroup>,
    shadow_sampler: wgpu::Sampler,
    dummy_evsm_moments: EvsmMomentTarget,
    shadow_cache_frame: u64,
}

struct EnvironmentRendererState {
    environment_layout: wgpu::BindGroupLayout,
    environment_bind_group: wgpu::BindGroup,
    environment_uniform_buffer: wgpu::Buffer,
    capture_environment_bind_group: wgpu::BindGroup,
    capture_environment_uniform_buffer: wgpu::Buffer,
    reflection_probe_buffer: wgpu::Buffer,
    reflection_prefilter_layout: wgpu::BindGroupLayout,
    reflection_prefilter_pipeline: wgpu::RenderPipeline,
    _environment_brdf_lut: GpuEnvironmentBrdfLut,
    environment_cubemap_pool: GpuEnvironmentCubemapPool,
    reflection_probe_selection_cache: HashMap<u64, CachedReflectionProbeSelection>,
    reflection_probe_spatial_index: ReflectionProbeSpatialIndex,
    reflection_probe_capture_states: HashMap<u64, ReflectionProbeCaptureState>,
    reflection_faces_captured_this_frame: u32,
    reflection_mips_filtered_this_frame: u32,
    reflection_probe_evictions_total: u64,
    environment_sky_enabled: bool,
    capture_sky_enabled: bool,
}

struct PostProcessRendererState {
    ssao_pipeline: wgpu::RenderPipeline,
    ssao_blur_pipeline: wgpu::RenderPipeline,
    ssao_composite_pipeline: wgpu::RenderPipeline,
    fxaa_pipeline: wgpu::RenderPipeline,
    post_process_copy_pipeline: wgpu::RenderPipeline,
    ssao_layout: wgpu::BindGroupLayout,
    ssao_blur_layout: wgpu::BindGroupLayout,
    ssao_composite_layout: wgpu::BindGroupLayout,
    custom_post_process_layout: wgpu::BindGroupLayout,
    ao_target: Option<AmbientOcclusionTarget>,
    ssao_uniform_buffer: wgpu::Buffer,
    custom_post_process_uniform_buffer: wgpu::Buffer,
    custom_post_process_uniform_buffers: Vec<wgpu::Buffer>,
    post_process_target_a: Option<GpuTextureResource>,
    post_process_target_b: Option<GpuTextureResource>,
    ssr_history: Option<GpuTextureResource>,
    ssr_history_valid: bool,
    previous_post_process_view_proj: Mat4,
    custom_post_process_pipelines: HashMap<String, wgpu::RenderPipeline>,
}

struct SceneGpuState {
    material_layout: wgpu::BindGroupLayout,
    camera_layout: wgpu::BindGroupLayout,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_sampler: wgpu::Sampler,
    screen_sampler: wgpu::Sampler,
    white_srgb_texture: GpuTextureResource,
    white_linear_texture: GpuTextureResource,
    black_linear_texture: GpuTextureResource,
    neutral_normal_texture: GpuTextureResource,
    render_texture_targets: HashMap<String, GpuRenderTextureTarget>,
    texture_cache: HashMap<(u64, bool), GpuTextureResource>,
    texture_cache_revisions: HashMap<(u64, bool), u64>,
    baked_lightmap_texture: Option<(u64, GpuTextureResource)>,
    geometry_buffer_cache: HashMap<u64, CachedGeometryBuffers>,
    scene_draw_cache: HashMap<u64, CachedSceneDraw>,
    scene_cache_frame: u64,
}

struct RenderPipelineState {
    default_pipeline: wgpu::RenderPipeline,
    default_double_sided_pipeline: wgpu::RenderPipeline,
    transparent_pipeline: wgpu::RenderPipeline,
    transparent_double_sided_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    capture_default_pipeline: wgpu::RenderPipeline,
    capture_default_double_sided_pipeline: wgpu::RenderPipeline,
    capture_transparent_pipeline: wgpu::RenderPipeline,
    capture_transparent_double_sided_pipeline: wgpu::RenderPipeline,
    capture_sky_pipeline: wgpu::RenderPipeline,
    outline_mask_pipeline: wgpu::RenderPipeline,
    outline_overlay_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    custom_modules: HashMap<String, wgpu::ShaderModule>,
    custom_pipelines: HashMap<String, wgpu::RenderPipeline>,
    custom_capture_pipelines: HashMap<String, wgpu::RenderPipeline>,
}

struct OptionalRendererState {
    #[cfg(feature = "profiler")]
    gpu_timestamp_profiler: Option<GpuTimestampProfiler>,
    #[cfg(feature = "egui_render")]
    egui_ctx: egui::Context,
    #[cfg(feature = "egui_render")]
    egui_renderer: egui_wgpu::Renderer,
    #[cfg(all(feature = "egui_render", feature = "profiler"))]
    profiler_sort_mode: u8,
    #[cfg(all(feature = "egui_render", feature = "profiler"))]
    profiler_include_overhead: bool,
    #[cfg(all(feature = "egui_render", feature = "profiler", feature = "wgpu_window"))]
    detached_profiler: Option<DetachedProfilerWindow>,
    #[cfg(all(feature = "egui_render", feature = "profiler", feature = "wgpu_window"))]
    detached_profiler_closed_by_user: bool,
}

/// Platform-neutral WGPU renderer shared by desktop and browser adapters.
///
/// The renderer owns the surface, GPU device, pipelines, caches, shadows,
/// environment lighting, reflection probes, and post-processing. Platform
/// adapters only provide a surface, drive input/events, and decide when frames
/// are requested.
pub struct WgpuRenderer {
    core: WgpuCoreState,
    shadows: ShadowRendererState,
    environment: EnvironmentRendererState,
    post_process: PostProcessRendererState,
    scene: SceneGpuState,
    pipelines: RenderPipelineState,
    optional: OptionalRendererState,
    #[cfg(feature = "render_2d")]
    canvas_2d: Canvas2DRendererState,
    #[cfg(feature = "wgpu_window")]
    desktop: Option<DesktopWindowState>,
}

/// Desktop `RenderTarget` adapter. Kept as an alias so existing applications do
/// not need to change their construction code while browser builds instantiate
/// the same renderer through `WgpuRenderer::from_surface`.
#[cfg(feature = "wgpu_window")]
pub type WgpuRenderTarget = WgpuRenderer;

// Renderer implementation modules live under a separate facade.
#[path = "wgpu_window/implementation.rs"]
mod implementation;
