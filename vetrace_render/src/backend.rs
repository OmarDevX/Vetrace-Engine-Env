use std::collections::HashMap;

use glam::{Mat4, Vec2, Vec3, Vec4};
use vetrace_core::backends::RenderBackend;
use vetrace_core::components::builtins::{GlobalTransform, Name, Transform};
use vetrace_core::engine::Engine;
use vetrace_core::{DebugTextOverlayPanel, Entity, InputState};
#[cfg(feature = "profiler")]
use vetrace_profiler::{ProfilerReport, ProfilerUiSettings};

use crate::baked_lighting::render_baked_lighting_for_object;
use crate::components::{Atmosphere, Bloom, CubemapHandle, BakedLightmapReceiver, BakedLightProbeReceiver, CustomShaderMaterial, DirectionalLight, EmissiveLightEmitter, Material, MeshHandle, ObjMesh, Outline, PointLight, PostProcessing, PrimitiveShape, ReflectionProbe, ReflectionProbeCaptureMode, ReflectionProbeCustomMaterialCaptureMode, ReflectionProbeInvalidationMode, ReflectionProbeParallaxMode, Renderable, RenderLayers, RenderTextureCamera, ScreenSpaceRect, ShadowMode, Shape, SpotLight, Sprite3D, VolumetricFog, ALL_RENDER_LAYERS};
#[cfg(feature = "render_2d")]
use crate::components::{CanvasItem2D, Sprite2D};
use crate::resources::{BakedLightingScene, Camera, CustomPostProcessPass, CustomPostProcessStack, EguiOverlayInputSnapshot, EguiOverlayKeyEvent, EguiOverlayKeyboardInputSnapshot, EguiOverlayPanel, EnvironmentCubemap, RenderAssets, RenderSettings, RenderStats, ScreenSpaceReflections};
#[cfg(feature = "render_2d")]
use crate::resources::Camera2D;
#[cfg(feature = "egui_render")]
use crate::resources::EguiToolRegistry;

#[path = "backend_target.rs"]
mod target;
#[path = "render_frame.rs"]
mod frame;
#[path = "frame_extraction/build_frame.rs"]
mod build_frame;
#[path = "frame_extraction/frame_resources.rs"]
mod frame_resources;
#[path = "frame_extraction/scene_extraction.rs"]
mod scene_extraction;
#[path = "frame_extraction/entity_environment.rs"]
mod entity_environment;
#[path = "frame_extraction/entity_ui.rs"]
mod entity_ui;
#[path = "frame_extraction/entity_renderables.rs"]
mod entity_renderables;
#[cfg(feature = "render_2d")]
#[path = "frame_extraction/entity_2d.rs"]
mod entity_2d;
#[path = "frame_extraction/reflection_signatures.rs"]
mod reflection_signatures;
#[path = "frame_extraction/post_processing.rs"]
mod post_processing;
#[path = "frame_extraction/emissive_lights.rs"]
mod emissive_lights;
#[cfg(feature = "egui_render")]
#[path = "backend_ui_extract.rs"]
mod ui_extract;
#[path = "backend_utils.rs"]
mod utils;

pub use build_frame::build_render_frame;
pub use frame::*;
pub use target::*;
pub use utils::{build_object_index, material_color, primitive_radius, project_to_screen};

use emissive_lights::*;
use entity_environment::*;
use entity_renderables::*;
#[cfg(feature = "render_2d")]
use entity_2d::*;
use entity_ui::*;
use frame_resources::*;
use post_processing::*;
use scene_extraction::*;
use reflection_signatures::*;
#[cfg(feature = "egui_render")]
use ui_extract::*;
use utils::{
    egui_input_snapshot_from_input, egui_keyboard_input_snapshot_from_input,
    egui_panel_from_debug_overlay, global_transform_for, global_transform_matrix,
    material_for, render_skin_for, rotated_direction,
};
