use super::*;

#[derive(Clone, Debug)]
pub struct RenderFrame {
    pub settings: RenderSettings,
    pub camera: Camera,
    #[cfg(feature = "render_2d")]
    pub camera_2d: Camera2D,
    pub render_texture_views: Vec<RenderTextureView>,
    pub objects: Vec<RenderObject>,
    pub sprites: Vec<RenderSprite>,
    #[cfg(feature = "render_2d")]
    pub sprites_2d: Vec<RenderSprite2D>,
    pub overlays: Vec<RenderOverlayRect>,
    #[cfg(feature = "egui_render")]
    pub world_ui: Vec<RenderWorldUiElement>,
    #[cfg(feature = "egui_render")]
    pub screen_ui: Vec<RenderScreenUiElement>,
    pub directional_lights: Vec<RenderDirectionalLight>,
    pub point_lights: Vec<RenderPointLight>,
    pub spot_lights: Vec<RenderSpotLight>,
    pub environment: Option<RenderEnvironment>,
    pub reflection_probes: Vec<RenderReflectionProbe>,
    /// Scene/light/environment signature shared by automatic probe invalidation.
    pub reflection_global_signature: u64,
    /// Per-render-layer object signatures. Probe invalidation combines only the
    /// layers selected by its capture include/exclude masks.
    pub reflection_layer_signatures: [u64; 32],
    pub atmosphere: Option<Atmosphere>,
    pub fog: Option<VolumetricFog>,
    pub post_processing: PostProcessing,
    pub custom_post_process_passes: Vec<CustomPostProcessPass>,
    pub egui_overlay: Option<EguiOverlayPanel>,
    pub egui_input: Option<EguiOverlayInputSnapshot>,
    pub egui_keyboard_input: Option<EguiOverlayKeyboardInputSnapshot>,
    #[cfg(feature = "egui_render")]
    pub egui_tools: Option<EguiToolRegistry>,
    #[cfg(feature = "profiler")]
    pub profiler_report: Option<ProfilerReport>,
    #[cfg(feature = "profiler")]
    pub profiler_ui_settings: Option<ProfilerUiSettings>,
}

#[derive(Clone, Debug)]
pub struct RenderEnvironment {
    pub primary: Option<CubemapHandle>,
    pub secondary: Option<CubemapHandle>,
    pub transition: f32,
    pub intensity: f32,
    pub rotation_radians: f32,
    pub draw_sky: bool,
    pub diffuse_ibl: bool,
    pub specular_ibl: bool,
}

#[derive(Clone, Debug)]
pub struct RenderReflectionProbe {
    pub entity: Entity,
    pub primary: Option<CubemapHandle>,
    pub secondary: Option<CubemapHandle>,
    pub transition: f32,
    pub world_to_probe: Mat4,
    pub half_extents: Vec3,
    pub capture_position_local: Vec3,
    pub blend_distance: f32,
    pub intensity: f32,
    pub priority: i32,
    pub parallax_mode: ReflectionProbeParallaxMode,
    pub capture_mode: ReflectionProbeCaptureMode,
    pub capture_resolution: u32,
    pub capture_near: f32,
    pub capture_far: f32,
    pub transition_seconds: f32,
    pub update_interval_seconds: f32,
    pub capture_revision: u32,
    pub capture_priority: i32,
    pub invalidation_mode: ReflectionProbeInvalidationMode,
    pub invalidation_delay_seconds: f32,
    pub capture_transparent: bool,
    pub capture_shadows: bool,
    pub capture_custom_materials: ReflectionProbeCustomMaterialCaptureMode,
    pub probe_to_world: Mat4,
    pub capture_position_world: Vec3,
    pub include_layers: u32,
    pub exclude_layers: u32,
    pub capture_include_layers: u32,
    pub capture_exclude_layers: u32,
}


#[derive(Clone, Debug)]
pub struct RenderTextureView {
    pub source_entity: Entity,
    pub target_name: String,
    pub width: u32,
    pub height: u32,
    pub clear_color: [f32; 4],
    pub layer_mask: u32,
    pub order: i32,
    pub camera: Camera,
}

#[derive(Clone, Debug)]
pub struct RenderDirectionalLight {
    /// World-space direction the light travels through the scene.
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub shadow_mode: ShadowMode,
}

#[derive(Clone, Debug)]
pub struct RenderPointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: Option<f32>,
    pub shadow_mode: ShadowMode,
}

#[derive(Clone, Debug)]
pub struct RenderSpotLight {
    pub position: Vec3,
    /// World-space direction the spot emits toward. glTF lights emit down local -Z.
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: Option<f32>,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
    pub shadow_mode: ShadowMode,
}

#[derive(Clone, Debug)]
pub struct RenderSkin {
    pub joint_matrices: Vec<Mat4>,
}

impl RenderSkin {
    pub fn signature(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for matrix in &self.joint_matrices {
            for value in matrix.to_cols_array() {
                hash ^= value.to_bits() as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }
}

#[derive(Clone, Debug)]
pub struct RenderObject {
    pub entity: Entity,
    pub name: Option<String>,
    pub transform: GlobalTransform,
    pub shape: Option<Shape>,
    pub mesh: Option<MeshHandle>,
    pub material: Material,
    pub custom_shader: Option<CustomShaderMaterial>,
    pub outline: Option<Outline>,
    pub skin: Option<RenderSkin>,
    pub geometry_revision: u64,
    pub render_layers: u32,
    pub(crate) baked_lightmap: Option<crate::baked_lighting::RenderBakedLightmap>,
    pub(crate) baked_probes: Option<crate::baked_lighting::RenderBakedProbes>,
}

#[derive(Clone, Debug)]
pub struct RenderSprite {
    pub entity: Entity,
    pub transform: GlobalTransform,
    pub sprite: Sprite3D,
    pub material: Material,
}

#[cfg(feature = "render_2d")]
#[derive(Clone, Debug)]
pub struct RenderSprite2D {
    pub entity: Entity,
    pub transform: GlobalTransform,
    pub canvas: CanvasItem2D,
    pub sprite: Sprite2D,
}

#[derive(Clone, Debug)]
pub struct RenderOverlayRect {
    pub entity: Entity,
    pub name: Option<String>,
    pub rect: ScreenSpaceRect,
    pub material: Material,
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Debug)]
pub struct RenderWorldUiElement {
    pub entity: Entity,
    pub slot: u8,
    pub world_position: Vec3,
    pub placement: RenderWorldUiPlacement,
    pub kind: RenderScreenUiKind,
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Debug)]
pub struct RenderWorldUiPlacement {
    pub screen_offset_px: Vec2,
    pub size_px: Vec2,
    pub max_distance: f32,
    pub z_order: i32,
    pub anchor: vetrace_ui::Anchor,
    pub background: Vec3,
    pub background_alpha: f32,
    pub padding_px: Vec2,
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Debug)]
pub struct RenderScreenUiElement {
    pub entity: Entity,
    pub slot: u8,
    pub rect: ScreenSpaceRect,
    pub kind: RenderScreenUiKind,
    pub style: RenderUiVisualStyle,
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Debug)]
pub enum RenderScreenUiKind {
    Label {
        text: String,
        font_size: f32,
        color: Vec3,
        align: vetrace_ui::TextAlign,
    },
    Panel {
        size_px: Vec2,
        background: Vec3,
        alpha: f32,
    },
    Button {
        text: String,
        size_px: Vec2,
        background: Vec3,
        alpha: f32,
        enabled: bool,
        hovered: bool,
        pressed: bool,
    },
    TextEditor {
        text: String,
        placeholder: String,
        size_px: Vec2,
        background: Vec3,
        alpha: f32,
        focused: bool,
        multiline: bool,
    },
    List {
        items: Vec<String>,
        selected: Option<usize>,
        size_px: Vec2,
    },
    ColorRect {
        size_px: Vec2,
        color: Vec3,
        alpha: f32,
    },
}

#[cfg(feature = "egui_render")]
#[derive(Clone, Copy, Debug)]
pub struct RenderUiVisualStyle {
    pub corner_radius: f32,
    pub border_width: f32,
    pub border_color: Vec3,
    pub border_alpha: f32,
    pub text_color: Vec3,
    pub text_alpha: f32,
    pub font_size: f32,
    pub hover_brightness: f32,
    pub pressed_darkness: f32,
    pub shadow_color: Vec3,
    pub shadow_alpha: f32,
    pub shadow_offset: Vec2,
}

#[cfg(feature = "egui_render")]
impl From<vetrace_ui::UIVisualStyle> for RenderUiVisualStyle {
    fn from(style: vetrace_ui::UIVisualStyle) -> Self {
        Self {
            corner_radius: style.corner_radius,
            border_width: style.border_width,
            border_color: style.border_color,
            border_alpha: style.border_alpha,
            text_color: style.text_color,
            text_alpha: style.text_alpha,
            font_size: style.font_size,
            hover_brightness: style.hover_brightness,
            pressed_darkness: style.pressed_darkness,
            shadow_color: style.shadow_color,
            shadow_alpha: style.shadow_alpha,
            shadow_offset: style.shadow_offset,
        }
    }
}
