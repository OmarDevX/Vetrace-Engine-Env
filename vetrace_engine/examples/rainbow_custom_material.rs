use std::collections::HashMap;
use vetrace_engine::app::{app, App};
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

fn evaluate_rainbow_material(
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    uv: vec2<f32>,
    params: CustomMaterialParams
) -> MaterialResult {
    var result: MaterialResult;
    let time = params.custom_float_4;
    let rainbow_factor = dot(hit_point, vec3<f32>(1.0, 0.0, 0.0)) * params.custom_float_1 + time * params.custom_float_2;
    let hue = fract(rainbow_factor);
    let rainbow_color = hsv_to_rgb(vec3<f32>(hue, 1.0, 1.0));
    result.base_color = rainbow_color;
    result.roughness = params.roughness;
    result.metallic = params.metallic;
    result.emission = rainbow_color * params.custom_float_3;
    return result;
}
"#;

struct RainbowExample;

impl App for RainbowExample {
    fn setup(&mut self, engine: &mut Engine) {
        engine.auto_register_component::<CustomMaterial>("Custom Material");
        let mut obj = Object::new([0.0, 0.0, 0.0], 1.0, [1.0, 1.0, 1.0], 0.5, 0.0, false);
        obj.is_cube = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            let mut params = HashMap::new();
            params.insert("roughness".to_string(), MaterialParameter::Float(0.2));
            params.insert("metallic".to_string(), MaterialParameter::Float(0.0));
            params.insert("rainbow_scale".to_string(), MaterialParameter::Float(1.0));
            params.insert("speed".to_string(), MaterialParameter::Float(1.0));
            params.insert("glow_strength".to_string(), MaterialParameter::Float(0.0));
            let custom = CustomMaterial {
                material_type: "rainbow".to_string(),
                shader_source: RAINBOW_WGSL.to_string(),
                parameters: params,
            };
            let e = actor.entity();
            engine.insert_custom_material(e, custom);
        }
    }

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("Custom Material Demo")
        .with_size(720, 720)
        .run(RainbowExample)?;
    Ok(())
}
