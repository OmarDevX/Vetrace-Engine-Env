use std::collections::HashMap;
use std::sync::Arc;
use vetrace_editor::EditorPlugin;
use vetrace_engine::app::{app, App};
use vetrace_engine::components::components::CameraAttachment;
use vetrace_engine::components::components::FreeFlightControls;
use vetrace_engine::components::components::Transform;
use vetrace_engine::gpu::{GpuTexture, TextureHandle};
use vetrace_engine::scene::object::Object;
use vetrace_engine::{CustomMaterial, Engine, MaterialParameter};
const RAINBOW_WGSL: &str = r#"
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;
    let c = v * s;
    let x = c * (1.0 - abs(fract(h * 0.5) * 2.0 - 1.0));
    let m = v - c;
    var rgb = vec3<f32>(0.0);
    if (h < 1.0) { rgb = vec3<f32>(c, x, 0.0); }
    else if (h < 2.0) { rgb = vec3<f32>(x, c, 0.0); }
    else if (h < 3.0) { rgb = vec3<f32>(0.0, c, x); }
    else if (h < 4.0) { rgb = vec3<f32>(0.0, x, c); }
    else if (h < 5.0) { rgb = vec3<f32>(x, 0.0, c); }
    else { rgb = vec3<f32>(c, 0.0, x); }
    return rgb + m;
}

fn evaluate_rainbow(
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    uv: vec2<f32>,
    params: CustomMaterialParams
) -> MaterialResult {
    var result: MaterialResult;
    let time = params.custom_floats.w;
    let rainbow_factor =
        dot(hit_point, vec3<f32>(1.0, 0.0, 0.0)) * params.custom_floats.x + time * params.custom_floats.y;
    let hue = fract(rainbow_factor);
    let rainbow_color = hsv_to_rgb(vec3<f32>(hue, 1.0, 1.0));
    let tex_color = textureSampleLevel(textures[params.texture_index], tex_sampler, uv, 0.0).rgb;
    result.base_color = tex_color * rainbow_color;
    result.roughness = params.base_props.x;
    result.metallic = params.base_props.y;
    result.emission = rainbow_color * params.custom_floats.z;

    // Fresnel term to keep edges opaque while the center remains transparent
    let ndotv = max(dot(normalize(normal), normalize(-view_dir)), 0.0);
    let fresnel = pow(1.0 - ndotv, 5.0);
    result.transparency = (1.0 - fresnel) * params.transparency_params.x;
    result.transmission = params.transparency_params.y;
    result.transmission_roughness = params.transparency_params.z;
    result.ior = params.transparency_params.w;
    result.subsurface = vec4<f32>(params.subsurface_params.x, params.subsurface_params.yzw);
    result.clearcoat = params.coat_aniso.xy;
    result.anisotropy = params.coat_aniso.zw;
    result.sheen = vec4<f32>(params.sheen_params.x, params.sheen_params.yzw);
    result.displacement = params.normal_disp.y;
    return result;
}
"#;

struct RainbowExample;

impl App for RainbowExample {
    fn setup(&mut self, engine: &mut Engine) {
        engine.auto_register_component::<CustomMaterial>("Custom Material");
        let mut obj = Object::new([0.0, 0.0, 0.0], 1.0, [1.0, 1.0, 1.0], 0.5, 0.0, false);
        // Use a sphere so the Fresnel transparency is visible on curved surfaces
        obj.is_cube = false;
        if let Some(actor) = engine.spawn_object_as_actor(obj) {
            let e = actor.entity();
            drop(actor);

            let mut params = HashMap::new();
            params.insert("roughness".to_string(), MaterialParameter::Float(0.2));
            params.insert("metallic".to_string(), MaterialParameter::Float(0.0));
            params.insert("rainbow_scale".to_string(), MaterialParameter::Float(1.0));
            params.insert("speed".to_string(), MaterialParameter::Float(1.0));
            params.insert("glow_strength".to_string(), MaterialParameter::Float(0.0));
            // Set up base transparency and transmission for the Fresnel effect
            params.insert("transparency".to_string(), MaterialParameter::Float(1.0));
            params.insert("transmission".to_string(), MaterialParameter::Float(1.0));
            params.insert(
                "transmission_roughness".to_string(),
                MaterialParameter::Float(0.0),
            );
            params.insert("refraction_ior".to_string(), MaterialParameter::Float(1.5));

            let img = image::open("assets/textures/tree.jpg").unwrap().to_rgba8();
            let (w, h) = img.dimensions();
            let tex = GpuTexture::from_rgba8(
                engine.renderer.device(),
                engine.renderer.queue(),
                img.as_raw(),
                w,
                h,
                true,
                "tree",
            )
            .unwrap();
            let tex_handle = TextureHandle(Arc::new(tex));
            params.insert(
                "texture".to_string(),
                MaterialParameter::Texture(tex_handle),
            );

            let custom = CustomMaterial {
                material_type: "rainbow".to_string(),
                shader_source: RAINBOW_WGSL.to_string(),
                parameters: params,
            };

            engine.insert_custom_material(e, custom);
        }
        // Independent camera entity with FreeFlightControls (RMB to move/rotate)
        let cam = engine.spawn_empty("camera");
        engine.world.insert(
            cam,
            Transform {
                position: [0.0, 0.0, -3.0],
                ..Default::default()
            },
        );
        engine.world.insert(cam, CameraAttachment::default());
        engine.world.insert(cam, FreeFlightControls::default());
    }

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("Custom Material Demo")
        .with_size(720, 720)
        .add_plugin(EditorPlugin::new())
        .run(RainbowExample)?;
    Ok(())
}
