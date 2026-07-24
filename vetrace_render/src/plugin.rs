use std::any::Any;
use std::error::Error;

use vetrace_core::app::Plugin;
use vetrace_core::Stage;
use vetrace_core::backends::RenderBackend;
use vetrace_core::engine::{ComponentManager, Engine};

use crate::backend::SceneRenderBackend;
use crate::components::*;
use crate::resources::{apply_reflection_probe_capture_requests, BakedLightingScene, EnvironmentCubemap, Camera, ReflectionProbeCaptureRequests, RenderAssets, RenderSettings, RenderStats, ScreenSpaceReflections};
#[cfg(feature = "render_2d")]
use crate::resources::Camera2D;

pub struct RenderPlugin {
    backend_factory: Box<dyn FnMut(&mut Engine) -> Box<dyn RenderBackend>>,
}

impl RenderPlugin {
    pub fn new() -> Self {
        #[cfg(feature = "wgpu_window")]
        {
            return Self::wgpu_window_from_settings();
        }
        #[cfg(all(not(feature = "wgpu_window"), feature = "sdl_window"))]
        {
            return Self::sdl_window_from_settings();
        }
        #[cfg(all(not(feature = "wgpu_window"), not(feature = "sdl_window")))]
        {
            Self::headless()
        }
    }

    pub fn headless() -> Self {
        Self { backend_factory: Box::new(|_engine| Box::new(SceneRenderBackend::headless())) }
    }

    pub fn with_backend<B, F>(mut factory: F) -> Self
    where
        B: RenderBackend,
        F: FnMut(&mut Engine) -> B + 'static,
    {
        Self { backend_factory: Box::new(move |engine| Box::new(factory(engine))) }
    }



    #[cfg(feature = "wgpu_window")]
    pub fn wgpu_window_from_settings() -> Self {
        Self {
            backend_factory: Box::new(move |engine| {
                let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
                match SceneRenderBackend::wgpu_window_with_settings(settings) {
                    Ok(backend) => Box::new(backend),
                    Err(err) => {
                        eprintln!("vetrace_render: failed to create WGPU render target: {err}; falling back to headless renderer");
                        Box::new(SceneRenderBackend::headless())
                    }
                }
            }),
        }
    }

    #[cfg(feature = "wgpu_window")]
    pub fn wgpu_window(title: impl Into<String>, width: u32, height: u32) -> Self {
        let title = title.into();
        Self {
            backend_factory: Box::new(move |_engine| match SceneRenderBackend::wgpu_window(title.clone(), width, height) {
                Ok(backend) => Box::new(backend),
                Err(err) => {
                    eprintln!("vetrace_render: failed to create WGPU render target: {err}; falling back to headless renderer");
                    Box::new(SceneRenderBackend::headless())
                }
            }),
        }
    }

    #[cfg(feature = "sdl_window")]
    pub fn sdl_window_from_settings() -> Self {
        Self {
            backend_factory: Box::new(move |engine| {
                let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
                match SceneRenderBackend::sdl_window(settings.title, settings.width, settings.height) {
                    Ok(backend) => Box::new(backend),
                    Err(err) => {
                        eprintln!("vetrace_render: failed to create SDL render target: {err}; falling back to headless renderer");
                        Box::new(SceneRenderBackend::headless())
                    }
                }
            }),
        }
    }

    #[cfg(feature = "sdl_window")]
    pub fn sdl_window(title: impl Into<String>, width: u32, height: u32) -> Self {
        let title = title.into();
        Self {
            backend_factory: Box::new(move |_engine| match SceneRenderBackend::sdl_window(title.clone(), width, height) {
                Ok(backend) => Box::new(backend),
                Err(err) => {
                    eprintln!("vetrace_render: failed to create SDL render target: {err}; falling back to headless renderer");
                    Box::new(SceneRenderBackend::headless())
                }
            }),
        }
    }
}


#[cfg(feature = "render_2d")]
pub struct Render2dPlugin;

#[cfg(feature = "render_2d")]
impl Render2dPlugin {
    pub fn new() -> Self { Self }
}

#[cfg(feature = "render_2d")]
impl Default for Render2dPlugin {
    fn default() -> Self { Self::new() }
}

#[cfg(feature = "render_2d")]
impl Plugin for Render2dPlugin {
    fn name(&self) -> &'static str { "render_2d" }

    fn dependencies(&self) -> Vec<&'static str> { vec!["render"] }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        if !engine.contains_resource::<Camera2D>() {
            engine.insert_resource(Camera2D::default());
        }
        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_reflected_named::<CanvasItem2D>(
                "vetrace.render.canvas_item_2d",
                "Canvas Item 2D",
                "2D Rendering",
            );
            cm.register_reflected_named::<Sprite2D>(
                "vetrace.render.sprite_2d",
                "Sprite 2D",
                "2D Rendering",
            );
            publish_enum_field::<BlendMode2D>(
                cm,
                "vetrace.render.canvas_item_2d",
                "blend_mode",
            )?;
            publish_enum_field::<TextureFilter2D>(
                cm,
                "vetrace.render.sprite_2d",
                "filter",
            )?;
            let _ = cm.register_alias("vetrace.render.sprite_2d", "Sprite2D");
            let _ = cm.register_alias("vetrace.render.canvas_item_2d", "CanvasItem2D");
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl Default for RenderPlugin {
    fn default() -> Self { Self::new() }
}

fn publish_enum_field<E: vetrace_core::VetraceEnum>(
    registry: &mut ComponentManager,
    component: &str,
    path: &str,
) -> Result<(), Box<dyn Error>> {
    registry
        .register_enum_field::<E>(component, path)
        .map_err(|error| -> Box<dyn Error> {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })
}

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str { "render" }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        if !engine.contains_resource::<RenderSettings>() {
            engine.insert_resource(RenderSettings::default());
        }
        if !engine.contains_resource::<Camera>() {
            engine.insert_resource(Camera::default());
        }
        if !engine.contains_resource::<RenderAssets>() {
            engine.insert_resource(RenderAssets::default());
        }
        if !engine.contains_resource::<RenderStats>() {
            engine.insert_resource(RenderStats::default());
        }
        if !engine.contains_resource::<EnvironmentCubemap>() {
            engine.insert_resource(EnvironmentCubemap::default());
        }
        if !engine.contains_resource::<ScreenSpaceReflections>() {
            engine.insert_resource(ScreenSpaceReflections::default());
        }
        if !engine.contains_resource::<BakedLightingScene>() {
            engine.insert_resource(BakedLightingScene::default());
        }
        if !engine.contains_resource::<ReflectionProbeCaptureRequests>() {
            engine.insert_resource(ReflectionProbeCaptureRequests::default());
        }
        let backend = (self.backend_factory)(engine);
        engine.insert_resource::<Box<dyn RenderBackend>>(backend);

        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_named::<MeshHandle>("vetrace.render.mesh_handle", "Mesh Handle");
            cm.register_named::<MaterialHandle>("vetrace.render.material_handle", "Material Handle");
            cm.register_reflected_transient_named::<Renderable>("vetrace.render.renderable", "Renderable", "Rendering");
            cm.register_named::<ObjMesh>("vetrace.render.obj_mesh", "OBJ Mesh");
            cm.register_reflected_named::<Material>("vetrace.render.material", "Material", "Rendering");
            cm.register_reflected_transient_named::<Shape>("vetrace.render.shape", "Shape", "Rendering");
            cm.register_reflected_named::<Sprite3D>("vetrace.render.sprite_3d", "Sprite 3D", "Rendering");
            cm.register_reflected_named::<ScreenSpaceRect>("vetrace.render.screen_space_rect", "Screen Space Rect", "Rendering");
            cm.register_reflected_named::<CustomShaderMaterial>("vetrace.render.custom_shader_material", "Custom Shader Material", "Rendering");
            cm.register_reflected_named::<RenderLayers>("vetrace.render.render_layers", "Render Layers", "Rendering");
            cm.register_reflected_named::<RenderTextureCamera>("vetrace.render.render_texture_camera", "Render Texture Camera", "Rendering");
            cm.register_reflected_named::<Outline>("vetrace.render.outline", "Outline", "Rendering");
            cm.register_reflected_transient_named::<CameraAttachment>("vetrace.render.camera_attachment", "Camera Attachment", "Rendering");
            cm.register_reflected_named::<Bloom>("vetrace.render.bloom", "Bloom", "Rendering");
            cm.register_reflected_named::<DepthOfField>("vetrace.render.depth_of_field", "Depth of Field", "Rendering");
            cm.register_reflected_named::<VolumetricFog>("vetrace.render.volumetric_fog", "Volumetric Fog", "Rendering");
            cm.register_reflected_named::<BakedLightmapReceiver>("vetrace.render.baked_lightmap_receiver", "Baked Lightmap Receiver", "Lighting");
            cm.register_reflected_named::<BakedLightProbeReceiver>("vetrace.render.baked_light_probe_receiver", "Baked Light Probe Receiver", "Lighting");
            cm.register_named::<BakedLightProbeDebugMarker>("vetrace.render.baked_light_probe_debug_marker", "Baked Light Probe Debug Marker");
            cm.register_reflected_named::<DirectionalLight>("vetrace.render.directional_light", "Directional Light", "Lighting");
            cm.register_reflected_named::<PointLight>("vetrace.render.point_light", "Point Light", "Lighting");
            cm.register_reflected_named::<EmissiveLightEmitter>("vetrace.render.emissive_light_emitter", "Emissive Light Emitter", "Lighting");
            cm.register_reflected_named::<BakedRectAreaLight>("vetrace.render.baked_rect_area_light", "Baked Rect Area Light", "Lighting");
            cm.register_reflected_named::<SpotLight>("vetrace.render.spot_light", "Spot Light", "Lighting");
            cm.register_reflected_named::<PostProcessing>("vetrace.render.post_processing", "Post Processing", "Rendering");
            cm.register_reflected_named::<CloudProfile>("vetrace.render.cloud_profile", "Cloud Profile", "Environment");
            cm.register_reflected_named::<VolumetricCloud>("vetrace.render.volumetric_cloud", "Volumetric Cloud", "Environment");
            cm.register_reflected_named::<Atmosphere>("vetrace.render.atmosphere", "Atmosphere", "Environment");
            cm.register_reflected_named::<ReflectionProbe>("vetrace.render.reflection_probe", "Reflection Probe", "Environment");

            // Enum metadata is owned by the rendering plugin. Studio and Lua
            // consume these options through the generic reflection registry.
            publish_enum_field::<AlphaMode>(cm, "vetrace.render.material", "alpha_mode")?;
            publish_enum_field::<PrimitiveShape>(cm, "vetrace.render.shape", "primitive")?;
            publish_enum_field::<CustomShaderVertexInterface>(cm, "vetrace.render.custom_shader_material", "vertex_interface")?;
            publish_enum_field::<CustomShaderCullMode>(cm, "vetrace.render.custom_shader_material", "cull_mode")?;
            publish_enum_field::<CustomShaderDepthCompare>(cm, "vetrace.render.custom_shader_material", "depth_compare")?;
            publish_enum_field::<CustomShaderRenderBucket>(cm, "vetrace.render.custom_shader_material", "render_bucket")?;
            publish_enum_field::<CustomShaderReflectionCaptureMode>(cm, "vetrace.render.custom_shader_material", "reflection_capture_mode")?;
            publish_enum_field::<ShadowMode>(cm, "vetrace.render.directional_light", "shadow_mode")?;
            publish_enum_field::<ShadowMode>(cm, "vetrace.render.point_light", "shadow_mode")?;
            publish_enum_field::<ShadowMode>(cm, "vetrace.render.spot_light", "shadow_mode")?;
            publish_enum_field::<ToneMapper>(cm, "vetrace.render.post_processing", "tone_mapper")?;
            publish_enum_field::<GlobalIlluminationMode>(cm, "vetrace.render.post_processing", "gi_mode")?;
            publish_enum_field::<RendererProfile>(cm, "vetrace.render.post_processing", "fallback_policy.profile")?;
            publish_enum_field::<ReflectionProbeParallaxMode>(cm, "vetrace.render.reflection_probe", "parallax_mode")?;
            publish_enum_field::<ReflectionProbeCaptureMode>(cm, "vetrace.render.reflection_probe", "capture_mode")?;
            publish_enum_field::<ReflectionProbeInvalidationMode>(cm, "vetrace.render.reflection_probe", "invalidation_mode")?;
            publish_enum_field::<ReflectionProbeCustomMaterialCaptureMode>(cm, "vetrace.render.reflection_probe", "capture_custom_materials")?;
            let _ = cm.register_alias("vetrace.render.material", "material");
            let _ = cm.register_alias("vetrace.render.point_light", "PointLight");
            let _ = cm.register_alias("vetrace.render.directional_light", "DirectionalLight");
            let _ = cm.register_alias("vetrace.render.spot_light", "SpotLight");
        }
        Ok(())
    }

    fn update_stage(&self) -> Stage { Stage::PostUpdate }

    fn update(&mut self, engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        apply_reflection_probe_capture_requests(engine);
        Ok(())
    }

    fn render(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        let _ = engine.with_resource_removed::<Box<dyn RenderBackend>, _>(
            |backend, engine| backend.render(engine),
        );
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enum_values(engine: &Engine, component: &str, field: &str) -> Vec<String> {
        engine
            .get_resource::<ComponentManager>()
            .unwrap()
            .descriptor(component)
            .unwrap()
            .schema
            .as_ref()
            .unwrap()
            .fields
            .iter()
            .find(|schema| schema.name == field)
            .unwrap()
            .enum_variants
            .clone()
    }

    #[cfg(feature = "render_2d")]
    #[test]
    fn render_2d_plugin_owns_2d_registration_and_camera() {
        let mut engine = Engine::new();
        RenderPlugin::headless().initialize(&mut engine).unwrap();
        assert!(engine
            .get_resource::<ComponentManager>()
            .unwrap()
            .descriptor("vetrace.render.sprite_2d")
            .is_none());

        Render2dPlugin::new().initialize(&mut engine).unwrap();
        assert!(engine.contains_resource::<Camera2D>());
        assert!(engine
            .get_resource::<ComponentManager>()
            .unwrap()
            .descriptor("vetrace.render.sprite_2d")
            .is_some());
        assert_eq!(
            enum_values(&engine, "vetrace.render.canvas_item_2d", "blend_mode"),
            vec!["Alpha".to_string(), "Additive".to_string(), "Multiply".to_string()],
        );
        assert_eq!(
            enum_values(&engine, "vetrace.render.sprite_2d", "filter"),
            vec!["Nearest".to_string(), "Linear".to_string()],
        );
    }

    #[test]
    fn render_plugin_publishes_enum_options_for_the_generic_inspector() {
        let mut engine = Engine::new();
        RenderPlugin::headless().initialize(&mut engine).unwrap();
        assert!(engine.contains_resource::<ScreenSpaceReflections>());

        assert_eq!(
            enum_values(&engine, "vetrace.render.material", "alpha_mode"),
            vec!["Opaque".to_string(), "Mask".to_string(), "Blend".to_string()],
        );
        assert_eq!(
            enum_values(&engine, "vetrace.render.shape", "primitive"),
            vec![
                "Cube".to_string(),
                "Sphere".to_string(),
                "Capsule".to_string(),
                "Plane".to_string(),
                "Quad".to_string(),
            ],
        );
        assert_eq!(
            enum_values(&engine, "vetrace.render.reflection_probe", "capture_mode"),
            vec![
                "Imported".to_string(),
                "Baked".to_string(),
                "OnDemand".to_string(),
                "Realtime".to_string(),
            ],
        );
    }
}
