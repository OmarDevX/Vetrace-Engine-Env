#[cfg(all(feature = "profiler", not(target_arch = "wasm32")))]
use std::time::Instant;
#[cfg(all(feature = "profiler", target_arch = "wasm32"))]
use web_time::Instant;

use super::*;

/// Active renderer backend used by default.
///
/// It intentionally avoids owning ECS or game-specific state. It consumes core
/// transforms and render crate components, builds a render list, and forwards it
/// to a platform target. The default target is headless, while the `sdl_window`
/// feature enables a simple SDL window target. The old monolithic WGPU renderer has been removed; new render work should stay
/// behind this render-frame/target boundary.

pub struct SceneRenderBackend {
    target: Box<dyn RenderTarget>,
}

impl SceneRenderBackend {
    pub fn headless() -> Self {
        Self { target: Box::new(HeadlessRenderTarget::default()) }
    }

    pub fn with_target(target: Box<dyn RenderTarget>) -> Self {
        Self { target }
    }

    #[cfg(feature = "wgpu_window")]
    pub fn wgpu_window(title: impl Into<String>, width: u32, height: u32) -> Result<Self, String> {
        Ok(Self { target: Box::new(crate::wgpu_window::WgpuRenderTarget::new(title, width, height)?) })
    }

    #[cfg(feature = "wgpu_window")]
    pub fn wgpu_window_with_settings(settings: crate::resources::RenderSettings) -> Result<Self, String> {
        Ok(Self {
            target: Box::new(crate::wgpu_window::WgpuRenderTarget::new_from_render_settings(settings)?),
        })
    }

    #[cfg(feature = "sdl_window")]
    pub fn sdl_window(title: impl Into<String>, width: u32, height: u32) -> Result<Self, String> {
        Ok(Self { target: Box::new(crate::sdl::SdlRenderTarget::new(title, width, height)?) })
    }
}

impl Default for SceneRenderBackend {
    fn default() -> Self { Self::headless() }
}

impl RenderBackend for SceneRenderBackend {
    fn render(&mut self, engine: &mut Engine) {
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        self.target.begin_frame(engine);
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("render.target_begin_frame", started.elapsed());

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        let frame = build_render_frame(engine);
        #[cfg(feature = "profiler")]
        {
            vetrace_profiler::record_timing("render.build_render_frame", started.elapsed());
            vetrace_profiler::record_counter("render.objects", frame.objects.len() as f64, "");
            vetrace_profiler::record_counter("render.sprites", frame.sprites.len() as f64, "");
            #[cfg(feature = "render_2d")]
            vetrace_profiler::record_counter("render.2d.extracted", frame.sprites_2d.len() as f64, "sprites");
            vetrace_profiler::record_counter("render.overlays", frame.overlays.len() as f64, "");
            vetrace_profiler::record_counter("render.directional_lights", frame.directional_lights.len() as f64, "");
            vetrace_profiler::record_counter("render.point_lights", frame.point_lights.len() as f64, "");
            vetrace_profiler::record_counter("render.spot_lights", frame.spot_lights.len() as f64, "");
        }

        let assets = engine.get_resource::<RenderAssets>();
        #[cfg(feature = "profiler")]
        let started = Instant::now();
        self.target.render(&frame, assets);
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("render.target_render", started.elapsed());

        #[cfg(feature = "profiler")]
        let started = Instant::now();
        if let Some(stats) = engine.get_resource_mut::<RenderStats>() {
            stats.frames_rendered = stats.frames_rendered.saturating_add(1);
            stats.visible_objects = frame.objects.len();
            #[cfg(feature = "render_2d")]
            {
                stats.visible_sprites_2d = frame.sprites_2d.len();
            }
            stats.directional_lights = frame.directional_lights.len();
            stats.point_lights = frame.point_lights.len();
            stats.spot_lights = frame.spot_lights.len();
            stats.has_atmosphere = frame.atmosphere.is_some();
            stats.has_fog = frame.fog.as_ref().map(|fog| fog.enabled).unwrap_or(false);
        }
        #[cfg(feature = "profiler")]
        vetrace_profiler::record_timing("render.update_stats", started.elapsed());
    }
}

/// Target abstraction used by the active render backend.
///
/// WGPU/SDL/software/headless implementations can share the same render-frame
/// extraction path instead of each reaching directly into ECS.
pub trait RenderTarget: 'static {
    /// Called before render-frame extraction. Window targets can pump input
    /// events here and write them into `vetrace_core::InputState` without
    /// making the core crate depend on any platform API.
    fn begin_frame(&mut self, _engine: &mut Engine) {}

    fn render(&mut self, frame: &RenderFrame, assets: Option<&RenderAssets>);
}

#[derive(Default)]
pub struct HeadlessRenderTarget {
    pub last_frame: Option<RenderFrame>,
}

impl RenderTarget for HeadlessRenderTarget {
    fn render(&mut self, frame: &RenderFrame, _assets: Option<&RenderAssets>) {
        self.last_frame = Some(frame.clone());
    }
}
