use crate::gpu::TextureHandle;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::ecs::Component;
use crate::inspector::Inspectable;
use crate::inspector::export::{ExportKind, ExportedField};
use vetrace_engine_macros::Inspectable;

pub const CUSTOM_MATERIAL_OUTPUT_SURFACE_COLOR: u32 = 1 << 0;
pub const CUSTOM_MATERIAL_OUTPUT_NORMALS: u32 = 1 << 1;
pub const CUSTOM_MATERIAL_OUTPUT_EMISSIVE: u32 = 1 << 2;
pub const CUSTOM_MATERIAL_OUTPUT_TRANSPARENCY: u32 = 1 << 3;
pub const CUSTOM_MATERIAL_OUTPUT_RAYTRACING_COMPATIBLE: u32 = 1 << 4;

pub const CUSTOM_MATERIAL_FLAG_RASTER_ONLY: u32 = 1 << 0;
pub const CUSTOM_MATERIAL_FLAG_FALLBACK_TO_RASTER_DATA: u32 = 1 << 1;

/// Shared material contract produced by raster custom materials and consumed by RT effect passes.
#[derive(Debug, Clone, Copy)]
pub struct MaterialOutputContract {
    pub base_color: [f32; 4],
    pub normal: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub emissive: [f32; 3],
    pub alpha: f32,
    pub transmission: f32,
    pub ior: f32,
    pub custom_flags: u32,
}

impl Default for MaterialOutputContract {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            roughness: 0.5,
            metallic: 0.0,
            emissive: [0.0, 0.0, 0.0],
            alpha: 1.0,
            transmission: 0.0,
            ior: 1.5,
            custom_flags: 0,
        }
    }
}

/// User-provided material with custom WGSL evaluation code.
#[derive(Debug, Clone, Inspectable)]
pub struct CustomMaterial {
    /// Identifier used to select the material implementation.
    #[export]
    pub material_type: String,
    /// Raw WGSL source for the material function.
    #[export]
    pub shader_source: String,
    /// Bitmask declaring which shared material-contract outputs the shader affects.
    #[export]
    pub output_flags: u32,
    /// Marks unsupported shaders as raster-only instead of RT-evaluable.
    #[export]
    pub raster_only: bool,
    /// Arbitrary parameters exposed to the material function.
    pub parameters: HashMap<String, MaterialParameter>,
}

impl Default for CustomMaterial {
    fn default() -> Self {
        Self {
            material_type: String::new(),
            shader_source: String::new(),
            output_flags: CUSTOM_MATERIAL_OUTPUT_SURFACE_COLOR
                | CUSTOM_MATERIAL_OUTPUT_RAYTRACING_COMPATIBLE,
            raster_only: false,
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
    Texture(TextureHandle),
    Bool(bool),
    /// Reference to the current screen color buffer.
    ScreenTexture,
}

impl CustomMaterial {
    pub fn affects_output(&self, flag: u32) -> bool {
        (self.output_flags & flag) != 0
    }

    pub fn is_rt_compatible(&self) -> bool {
        !self.raster_only && self.affects_output(CUSTOM_MATERIAL_OUTPUT_RAYTRACING_COMPATIBLE)
    }

    pub fn validation_warnings(&self) -> Vec<&'static str> {
        let mut warnings = Vec::new();
        if self.affects_output(CUSTOM_MATERIAL_OUTPUT_NORMALS)
            || self.affects_output(CUSTOM_MATERIAL_OUTPUT_TRANSPARENCY)
            || !self.is_rt_compatible()
        {
            warnings.push("This custom material uses features unavailable in RT reflections.");
        }
        if !self.is_rt_compatible() || self.raster_only {
            warnings.push("This material will fallback to raster data in RT pass.");
        }
        warnings
    }
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
        let mut inserted = HashSet::new();
        for material_name in used_materials {
            if inserted.insert(material_name.clone()) {
                if let Some(code) = self.material_registry.get(material_name) {
                    material_functions.push_str(code);
                    material_functions.push('\n');
                }
            }
        }
        shader_source =
            shader_source.replace("// MATERIAL_FUNCTIONS_PLACEHOLDER", &material_functions);

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
