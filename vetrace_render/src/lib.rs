//! Vetrace Render.
//!
//! Active rendering is exposed as a normal Vetrace plugin. The default backend
//! is headless and deterministic; enabling `wgpu_render` exposes the shared WGPU
//! renderer, while `wgpu_window` adds the native desktop window adapter. The optional `sdl_window` feature remains only as a
//! software fallback/debug target. The old monolithic renderer has been removed
//! from this crate; active rendering goes through this backend/target path.

pub mod actor_ext;
pub mod backend;
pub mod baked_lighting;
mod baked_lighting_bake;
mod baked_lighting_geometry;
pub mod components;
pub mod plugin;
pub mod resources;

#[cfg(feature = "gltf_loader")]
pub mod gltf_loader;

#[cfg(feature = "sdl_window")]
pub mod sdl;

#[cfg(feature = "wgpu_render")]
pub mod wgpu_backend;

#[cfg(feature = "wgpu_render")]
pub mod wgpu_window;

pub use actor_ext::{RenderActorExt, RenderBundle};
#[cfg(feature = "render_2d")]
pub use actor_ext::Sprite2DBundle;
pub use baked_lighting::{
    bake_and_save_baked_lighting, baked_lighting_debug_mode,
    cycle_baked_lighting_debug_mode, load_baked_lighting,
    set_baked_lighting_debug_mode, set_baked_lighting_runtime_mode,
    baked_lighting_runtime_mode, unload_baked_lighting,
    BakedLightingBakeConfig, BakedLightingBakeReport,
};
pub use backend::{build_render_frame, build_object_index, material_color, primitive_radius, project_to_screen, HeadlessRenderTarget, RenderDirectionalLight, RenderEnvironment, RenderFrame, RenderObject, RenderOverlayRect, RenderPointLight, RenderReflectionProbe, RenderSprite, RenderSpotLight, RenderTarget, RenderTextureView, SceneRenderBackend};
#[cfg(feature = "render_2d")]
pub use backend::RenderSprite2D;

#[cfg(feature = "egui_render")]
pub use backend::{RenderScreenUiElement, RenderScreenUiKind, RenderWorldUiElement, RenderWorldUiPlacement};
pub use components::*;
pub use plugin::RenderPlugin;
#[cfg(feature = "render_2d")]
pub use plugin::Render2dPlugin;
pub use resources::{
    AdapterPreference, AmbientOcclusionMode, AntiAliasingMode, BakedLightingDebugMode,
    BakedLightingRuntimeMode, BakedLightingScene, BakedLightmapRegion, BakedProbeGrid, BakedProbeSample,
    Camera, CustomPostProcessPass, CustomPostProcessStack, EguiOverlayPanel,
    FreeFlightCameraController, ReflectionProbeCaptureRequests, ScreenSpaceReflections, SCREEN_SPACE_REFLECTIONS_PASS_ID,
    CubemapAsset, EnvironmentCubemap, MeshAsset, MeshVertex, PostProcessInput, PresentModePreference, RenderAssets,
    RenderSettings, RenderStats, ShadowFilterMode, TextureAsset,
};

#[cfg(feature = "render_2d")]
pub use resources::Camera2D;

#[cfg(feature = "egui_render")]
pub use resources::{EguiTool, EguiToolContext, EguiToolLayer, EguiToolRegistry};

#[cfg(feature = "egui_render")]
pub use egui;

#[cfg(feature = "gltf_loader")]
pub use gltf_loader::{load_gltf_actor, load_gltf_scene, load_gltf_scene_with_options, load_gltf_static_map, load_gltf_static_map_actor, GltfCollisionPolicy, GltfLoadOptions, GltfLoadReport};

#[cfg(feature = "wgpu_render")]
pub use wgpu_backend::{WgpuCustomShaderCache, WgpuCustomShader, WgpuOutlinePass, CustomShaderUniform, OutlineUniform};

#[cfg(feature = "wgpu_render")]
pub use wgpu_window::WgpuRenderer;

#[cfg(feature = "wgpu_window")]
pub use wgpu_window::WgpuRenderTarget;


