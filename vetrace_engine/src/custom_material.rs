use std::collections::HashMap;
use std::sync::Arc;

use crate::ecs::Component;
use crate::inspector::Inspectable;
use vetrace_engine_macros::Inspectable;
use crate::inspector::export::{ExportedField, ExportKind};

/// User-provided material with custom WGSL evaluation code.
#[derive(Debug, Clone, Inspectable)]
pub struct CustomMaterial {
    /// Identifier used to select the material implementation.
    #[export]
    pub material_type: String,
    /// Raw WGSL source for the material function.
    #[export]
    pub shader_source: String,
    /// Arbitrary parameters exposed to the material function.
    pub parameters: HashMap<String, MaterialParameter>,
}

impl Default for CustomMaterial {
    fn default() -> Self {
        Self {
            material_type: String::new(),
            shader_source: String::new(),
            parameters: HashMap::new(),
        }
    }
}

/// Generic parameter that can be passed to a [`CustomMaterial`].
#[derive(Debug, Clone)]
pub enum MaterialParameter {
    Float(f32),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Texture(u32),
    Bool(bool),
}

impl Component for CustomMaterial {}

/// Helper responsible for compiling ray tracing shaders with injected
/// custom material functions.
pub struct RaytraceShaderCompiler {
    /// Device used to create shader modules.
    pub device: Arc<wgpu::Device>,
    /// WGSL template containing placeholders for material functions and dispatch code.
    pub base_shader_template: String,
    /// Registry mapping material names to their WGSL implementations.
    pub material_registry: HashMap<String, String>,
}

impl RaytraceShaderCompiler {
    /// Register a new material function by name.
    pub fn register_material(&mut self, name: String, wgsl_code: String) {
        self.material_registry.insert(name, wgsl_code);
    }

    /// Compile a shader module containing only the materials referenced by `used_materials`.
    pub fn compile_shader(&self, used_materials: &[String]) -> Result<wgpu::ShaderModule, String> {
        let mut shader_source = self.base_shader_template.clone();

        let mut material_functions = String::new();
        for material_name in used_materials {
            if let Some(code) = self.material_registry.get(material_name) {
                material_functions.push_str(code);
                material_functions.push('\n');
            }
        }
        shader_source = shader_source.replace("// MATERIAL_FUNCTIONS_PLACEHOLDER", &material_functions);

        let dispatcher = self.generate_material_dispatcher(used_materials);
        shader_source = shader_source.replace(
            "    // MATERIAL_EVALUATION_PLACEHOLDER\n    return default_material_result(hit_point, normal, view_dir, uv);",
            &dispatcher,
        );

        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Custom Raytracer"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        Ok(shader_module)
    }

    fn generate_material_dispatcher(&self, materials: &[String]) -> String {
        let mut dispatcher = String::from("    switch material_id {\n");
        for (i, material_name) in materials.iter().enumerate() {
            dispatcher.push_str(&format!(
                "        case {}u: {{ return evaluate_{}(hit_point, normal, view_dir, uv, custom_materials[material_id]); }}\n",
                i, material_name
            ));
        }
        dispatcher.push_str(
            "        default: { return default_material_result(hit_point, normal, view_dir, uv); }\n    }\n",
        );
        dispatcher
    }
}

