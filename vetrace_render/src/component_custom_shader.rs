use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum CustomShaderVertexInterface {
    /// Backward-compatible interface: world position and normal only.
    /// Fragment locations: 0..=1.
    Legacy,
    /// Textured interface: world position, normal, and primary UV.
    /// Fragment locations: 0..=2. This is the common choice for screens,
    /// portals, mirrors, decals, and simple textured custom materials.
    Textured,
    /// Full material interface: world position, normal, UV, vertex color,
    /// tangent, and lightmap UV. Fragment locations: 0..=5. The fragment
    /// shader must declare every location because WGPU 0.20 requires the
    /// inter-stage interface to match exactly.
    Full,
}

impl Default for CustomShaderVertexInterface {
    fn default() -> Self { Self::Legacy }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum CustomShaderCullMode {
    /// Do not cull either triangle side. This preserves the historical
    /// CustomShaderMaterial behavior.
    None,
    Front,
    Back,
}

impl Default for CustomShaderCullMode {
    fn default() -> Self { Self::None }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum CustomShaderDepthCompare {
    Less,
    LessEqual,
    Always,
}

impl Default for CustomShaderDepthCompare {
    fn default() -> Self { Self::LessEqual }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum CustomShaderRenderBucket {
    /// Draw with normal opaque scene objects unless overridden by material alpha.
    Opaque,
    /// Draw after opaque scene objects, sorted back-to-front.
    Transparent,
    /// Draw after transparent objects. Useful for game-side highlight/outline shells.
    Overlay,
}

impl Default for CustomShaderRenderBucket {
    fn default() -> Self { Self::Opaque }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum CustomShaderReflectionCaptureMode {
    /// Do not render this custom material into reflection captures.
    Exclude,
    /// Render the object's standard `Material` as a safe generic fallback.
    MaterialFallback,
    /// Compile the custom fragment shader against the linear HDR capture target.
    Shader,
}

impl Default for CustomShaderReflectionCaptureMode {
    fn default() -> Self { Self::MaterialFallback }
}

/// Backend-agnostic custom shader hook.
///
/// GPU renderers can compile `wgsl_source`/`asset_path` and bind `params`;
/// fallback/software renderers use `fallback_color_a/b` so games can still
/// exercise the same material ownership path without changing core.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomShaderMaterial {
    pub shader_id: String,
    pub asset_path: Option<String>,
    pub wgsl_source: Option<String>,
    pub params: Vec<f32>,
    pub fallback_color_a: Vec3,
    pub fallback_color_b: Vec3,
    /// Selects which shared vertex-output contract feeds the fragment shader.
    /// Legacy preserves existing game shaders. Textured adds UV at location 2.
    /// Full additionally provides color, tangent, and lightmap UV at locations
    /// 3 through 5. The chosen vertex outputs must exactly match the fragment
    /// shader inputs on WGPU 0.20.
    #[serde(default)]
    pub vertex_interface: CustomShaderVertexInterface,
    /// Generic GPU pipeline culling override for custom materials.
    #[serde(default)]
    pub cull_mode: CustomShaderCullMode,
    /// Whether this custom material writes to the scene depth buffer.
    #[serde(default = "default_custom_shader_depth_write")]
    pub depth_write: bool,
    /// Depth test used by the custom material pipeline.
    #[serde(default)]
    pub depth_compare: CustomShaderDepthCompare,
    /// Which scene pass/bucket should draw this custom material.
    #[serde(default)]
    pub render_bucket: CustomShaderRenderBucket,
    /// Up to four named textures produced by `RenderTextureCamera` entities.
    /// Entry 0 maps to group 0 / binding 11, entry 1 to binding 12, through
    /// binding 14. Missing names use a safe black fallback texture.
    #[serde(default)]
    pub render_textures: Vec<String>,
    /// Controls how this material appears in reflection-probe captures.
    #[serde(default)]
    pub reflection_capture_mode: CustomShaderReflectionCaptureMode,
    /// Optional capture-only WGSL fragment source. When absent, `Shader` mode
    /// reuses the normal custom fragment source.
    #[serde(default)]
    pub reflection_capture_wgsl_source: Option<String>,
    /// Optional capture-only WGSL file path.
    #[serde(default)]
    pub reflection_capture_asset_path: Option<String>,
}


fn default_custom_shader_depth_write() -> bool { true }

impl Default for CustomShaderMaterial {
    fn default() -> Self {
        Self {
            shader_id: "default".to_string(),
            asset_path: None,
            wgsl_source: None,
            params: Vec::new(),
            fallback_color_a: Vec3::ONE,
            fallback_color_b: Vec3::splat(0.25),
            vertex_interface: CustomShaderVertexInterface::default(),
            cull_mode: CustomShaderCullMode::default(),
            depth_write: true,
            depth_compare: CustomShaderDepthCompare::default(),
            render_bucket: CustomShaderRenderBucket::default(),
            render_textures: Vec::new(),
            reflection_capture_mode: CustomShaderReflectionCaptureMode::MaterialFallback,
            reflection_capture_wgsl_source: None,
            reflection_capture_asset_path: None,
        }
    }
}
