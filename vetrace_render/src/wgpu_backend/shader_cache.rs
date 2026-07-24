use super::*;
use crate::components::CustomShaderMaterial;
use crate::resources::{RenderAssets, RenderSettings};

// Split-out implementation details for `wgpu_backend.rs`.

#[derive(Debug)]
pub enum WgpuShaderError {
    MissingSource { shader_id: String },
    Io { shader_id: String, path: String, error: std::io::Error },
}

impl std::fmt::Display for WgpuShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSource { shader_id } => write!(f, "custom shader `{shader_id}` has no WGSL source or asset_path"),
            Self::Io { shader_id, path, error } => write!(f, "failed to load custom shader `{shader_id}` from `{path}`: {error}"),
        }
    }
}

impl std::error::Error for WgpuShaderError {}

pub struct WgpuCustomShader {
    pub shader_id: String,
    pub module: wgpu::ShaderModule,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Default)]
pub struct WgpuCustomShaderCache {
    shaders: HashMap<String, WgpuCustomShader>,
}

impl WgpuCustomShaderCache {
    pub fn new() -> Self { Self::default() }

    pub fn get(&self, shader_id: &str) -> Option<&WgpuCustomShader> {
        self.shaders.get(shader_id)
    }

    pub fn prepare_material(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material: &CustomShaderMaterial,
        settings: &RenderSettings,
    ) -> Result<&WgpuCustomShader, WgpuShaderError> {
        self.prepare_material_with_assets(device, queue, material, settings, None)
    }

    /// Browser-safe variant that resolves path-backed WGSL from the ordinary
    /// `RenderAssets` resource before falling back to native filesystem I/O.
    pub fn prepare_material_with_assets(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material: &CustomShaderMaterial,
        settings: &RenderSettings,
        assets: Option<&RenderAssets>,
    ) -> Result<&WgpuCustomShader, WgpuShaderError> {
        let shader_id = material.shader_id.clone();
        if !self.shaders.contains_key(&shader_id) {
            let source = load_shader_source(material, assets)?;
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("vetrace custom shader: {shader_id}")),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
            });
            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(&format!("vetrace custom shader params layout: {shader_id}")),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("vetrace custom shader pipeline layout: {shader_id}")),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });
            let uniform = CustomShaderUniform::from_material(material, settings);
            let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("vetrace custom shader params: {shader_id}")),
                size: std::mem::size_of::<CustomShaderUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniform));
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("vetrace custom shader bind group: {shader_id}")),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() }],
            });
            self.shaders.insert(shader_id.clone(), WgpuCustomShader { shader_id: shader_id.clone(), module, bind_group_layout, pipeline_layout, uniform_buffer, bind_group });
        } else if let Some(shader) = self.shaders.get(&shader_id) {
            let uniform = CustomShaderUniform::from_material(material, settings);
            queue.write_buffer(&shader.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
        }
        Ok(self.shaders.get(&shader_id).expect("shader was just inserted or updated"))
    }
}

fn load_shader_source(
    material: &CustomShaderMaterial,
    assets: Option<&RenderAssets>,
) -> Result<String, WgpuShaderError> {
    if let Some(source) = &material.wgsl_source {
        return Ok(source.clone());
    }
    if let Some(path) = &material.asset_path {
        if let Some(source) = assets.and_then(|assets| assets.text_asset(path)) {
            return Ok(source.to_string());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            return fs::read_to_string(Path::new(path)).map_err(|error| WgpuShaderError::Io {
                shader_id: material.shader_id.clone(),
                path: path.clone(),
                error,
            });
        }
    }
    Err(WgpuShaderError::MissingSource { shader_id: material.shader_id.clone() })
}
