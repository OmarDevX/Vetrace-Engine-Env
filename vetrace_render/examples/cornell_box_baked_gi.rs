//! Cornell Box validation scene for Vetrace baked lightmaps and light probes.
//!
//! From the workspace root, bake once:
//! `cargo run -p vetrace_render --example cornell_box_baked_gi --features wgpu_window -- --bake-lighting`
//!
//! Then run without rebaking:
//! `cargo run -p vetrace_render --example cornell_box_baked_gi --features wgpu_window`
//!
//! Controls:
//! - B: Off -> Lightmap -> LightmapUv -> Probes -> Off
//! - M: toggle pure baked lighting / hybrid realtime direct + baked indirect GI
//! - J / K: decrease / increase exposure
//! - T: cycle tone mapper (Off -> ACES -> Neutral -> Reinhard)
//! - Space: pause/resume the moving probe-test sphere
//! - Escape: quit

use std::error::Error;
use std::f32::consts::{FRAC_PI_2, PI};
use std::path::{Path, PathBuf};

use glam::{Quat, Vec3};
use vetrace_core::{Actor, App, AppBuilder, Engine, InputState, Transform};
use vetrace_render::{
    bake_and_save_baked_lighting, baked_lighting_runtime_mode,
    cycle_baked_lighting_debug_mode, load_baked_lighting,
    set_baked_lighting_runtime_mode, AdapterPreference, BakedLightingBakeConfig,
    BakedLightingRuntimeMode, BakedLightProbeDebugMarker, BakedLightProbeReceiver,
    BakedLightmapReceiver, BakedRectAreaLight, Camera, Material, PostProcessing,
    PrimitiveShape, RenderBundle, RenderPlugin, RenderSettings, Renderable,
    ShadowFilterMode, Shape, ToneMapper,
};

const ROOM_WIDTH: f32 = 5.0;
const ROOM_HEIGHT: f32 = 5.0;
const ROOM_DEPTH: f32 = 5.0;
const CORNELL_AREA_LIGHT_INTENSITY: f32 = 38.0;
const CORNELL_AREA_LIGHT_SAMPLES: u32 = 49;
const CORNELL_INDIRECT_BOUNCES: u32 = 5;
const CORNELL_INDIRECT_BOUNCE_DECAY: f32 = 0.70;
const CORNELL_INDIRECT_INTENSITY: f32 = 1.22;
const CORNELL_LIGHTMAP_INTENSITY: f32 = 1.02;
const CORNELL_PROBE_INTENSITY: f32 = 1.18;

include!("cornell_box_baked_gi/options.rs");
include!("cornell_box_baked_gi/app.rs");
include!("cornell_box_baked_gi/scene.rs");

fn main() -> Result<(), Box<dyn Error>> {
    let Some(options) = CornellOptions::parse()? else {
        return Ok(());
    };

    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Cornell Box - Baked GI".to_string(),
            width: 1280,
            height: 800,
            clear_color: [0.006, 0.006, 0.009, 1.0],
            cursor_grab: false,
            cursor_visible: true,
            draw_bounds: false,
            adapter_preference: AdapterPreference::HighPerformance,
            shadow_map_size: 2048,
            shadow_filter_mode: ShadowFilterMode::Pcss,
            shadow_pcf_quality: 3,
            shadow_soft_radius: 2.5,
            shadow_pcss_light_radius: 3.0,
            shadow_max_distance: 30.0,
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .run_until_stopped(CornellBoxExample::new(options), None, 1.0 / 60.0)
}
