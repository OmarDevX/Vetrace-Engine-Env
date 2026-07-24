//! Vetrace-compatible vendored `egui-gizmo`.
//!
//! This is the upstream egui-gizmo implementation adapted to the engine's
//! active egui version so the map builder can pass the real `egui::Ui` type.

use std::cmp::Ordering;
use std::f32::consts::PI;
use std::hash::Hash;
use std::ops::Sub;

use crate::math::{screen_to_world, world_to_screen};
use egui::{Color32, Context, Id, PointerButton, Pos2, Rect, Sense, Ui, Vec2};
use glam::{DMat4, DQuat, DVec3, Mat4, Quat, Vec3, Vec4Swizzles};

fn quat_to_dquat(q: Quat) -> DQuat {
    DQuat::from_xyzw(q.x as f64, q.y as f64, q.z as f64, q.w as f64)
}

fn dquat_to_quat(q: DQuat) -> Quat {
    Quat::from_xyzw(q.x as f32, q.y as f32, q.z as f32, q.w as f32)
}

use crate::subgizmo::rotation::RotationParams;
use crate::subgizmo::scale::ScaleParams;
use crate::subgizmo::translation::TranslationParams;
use crate::subgizmo::{ArcballSubGizmo, RotationSubGizmo, ScaleSubGizmo, SubGizmo, TransformKind, TranslationSubGizmo};

mod math;
mod painter;
mod subgizmo;

pub const DEFAULT_SNAP_ANGLE: f32 = PI / 32.0;
pub const DEFAULT_SNAP_DISTANCE: f32 = 0.1;
pub const DEFAULT_SNAP_SCALE: f32 = 0.1;

pub struct Gizmo {
    id: Id,
    config: GizmoConfig,
    subgizmos: Vec<Box<dyn SubGizmo>>,
}

impl Gizmo {
    pub fn new(id_source: impl Hash) -> Self {
        Self { id: Id::new(id_source), config: GizmoConfig::default(), subgizmos: Default::default() }
    }

    pub fn model_matrix(mut self, model_matrix: mint::ColumnMatrix4<f32>) -> Self { self.config.model_matrix = Mat4::from(model_matrix).as_dmat4(); self }
    pub fn view_matrix(mut self, view_matrix: mint::ColumnMatrix4<f32>) -> Self { self.config.view_matrix = Mat4::from(view_matrix).as_dmat4(); self }
    pub fn projection_matrix(mut self, projection_matrix: mint::ColumnMatrix4<f32>) -> Self { self.config.projection_matrix = Mat4::from(projection_matrix).as_dmat4(); self }
    pub const fn viewport(mut self, viewport: Rect) -> Self { self.config.viewport = viewport; self }
    /// Restrict pointer hit-testing without changing the projection viewport.
    /// This is useful when the 3D render covers a full window behind docked UI.
    pub const fn interaction_rect(mut self, interaction_rect: Rect) -> Self { self.config.interaction_rect = Some(interaction_rect); self }
    pub const fn mode(mut self, mode: GizmoMode) -> Self { self.config.mode = mode; self }
    pub const fn orientation(mut self, orientation: GizmoOrientation) -> Self { self.config.orientation = orientation; self }
    pub const fn snapping(mut self, snapping: bool) -> Self { self.config.snapping = snapping; self }
    pub const fn snap_angle(mut self, snap_angle: f32) -> Self { self.config.snap_angle = snap_angle; self }
    pub const fn snap_distance(mut self, snap_distance: f32) -> Self { self.config.snap_distance = snap_distance; self }
    pub const fn snap_scale(mut self, snap_scale: f32) -> Self { self.config.snap_scale = snap_scale; self }
    pub const fn visuals(mut self, visuals: GizmoVisuals) -> Self { self.config.visuals = visuals; self }

    pub fn interact(mut self, ui: &mut Ui) -> Option<GizmoResult> {
        self.config.prepare(ui);
        match self.config.mode {
            GizmoMode::Rotate => {
                self.add_subgizmos(self.new_rotation());
                self.add_subgizmos(self.new_arcball());
            }
            GizmoMode::Translate => self.add_subgizmos(self.new_translation()),
            GizmoMode::Scale => self.add_subgizmos(self.new_scale()),
            GizmoMode::Omni => {
                self.add_subgizmos(self.new_translation());
                self.add_subgizmos(self.new_rotation());
                self.add_subgizmos(self.new_arcball());
                self.add_subgizmos(self.new_scale());
            }
        }
        let mut result = None;
        let mut active_subgizmo = None;
        let mut state = GizmoState::load(ui.ctx(), self.id);
        if let Some(pointer_ray) = self.pointer_ray(ui, state.active_subgizmo_id.is_some()) {
            let id = self.id;
            if state.active_subgizmo_id.is_none() {
                if let Some(subgizmo) = self.pick_subgizmo(ui, pointer_ray) {
                    subgizmo.set_focused(true);
                    // `pick_subgizmo` already proved that the pointer is over
                    // an actual handle. Register the drag only around that
                    // pointer location instead of making the complete viewport
                    // an interactive widget, which would steal input from
                    // editor panels layered over the 3D view.
                    let interaction_rect = Rect::from_center_size(pointer_ray.screen_pos, Vec2::splat(16.0));
                    let interaction = ui.interact(interaction_rect, id, Sense::click_and_drag());
                    let dragging = interaction.dragged_by(PointerButton::Primary);
                    if interaction.drag_started() && dragging {
                        state.active_subgizmo_id = Some(subgizmo.id());
                    }
                }
            }
            active_subgizmo = state.active_subgizmo_id.and_then(|id| self.subgizmos.iter_mut().find(|subgizmo| subgizmo.id() == id));
            if let Some(subgizmo) = active_subgizmo.as_mut() {
                if ui.input(|i| i.pointer.primary_down()) {
                    subgizmo.set_active(true);
                    subgizmo.set_focused(true);
                    result = subgizmo.update(ui, pointer_ray);
                } else {
                    state.active_subgizmo_id = None;
                }
            }
        }
        if let Some((_, result)) = active_subgizmo.zip(result) {
            self.config.translation = Vec3::from(result.translation).as_dvec3();
            self.config.rotation = quat_to_dquat(Quat::from(result.rotation));
            self.config.scale = Vec3::from(result.scale).as_dvec3();
        }
        state.save(ui.ctx(), self.id);
        self.draw_subgizmos(ui, &mut state);
        result
    }

    fn draw_subgizmos(&mut self, ui: &mut Ui, state: &mut GizmoState) {
        for subgizmo in &mut self.subgizmos {
            if state.active_subgizmo_id.is_none() || subgizmo.is_active() { subgizmo.draw(ui); }
        }
    }

    fn pick_subgizmo(&mut self, ui: &Ui, ray: Ray) -> Option<&mut Box<dyn SubGizmo>> {
        self.subgizmos.iter_mut().filter_map(|subgizmo| subgizmo.pick(ui, ray).map(|t| (t, subgizmo))).min_by(|(first, _), (second, _)| first.partial_cmp(second).unwrap_or(Ordering::Equal)).map(|(_, subgizmo)| subgizmo)
    }

    fn new_arcball(&self) -> [ArcballSubGizmo; 1] { [ArcballSubGizmo::new(self.id.with("arc"), self.config, ())] }
    fn new_rotation(&self) -> [RotationSubGizmo; 4] {
        [
            RotationSubGizmo::new(self.id.with("rx"), self.config, RotationParams { direction: GizmoDirection::X }),
            RotationSubGizmo::new(self.id.with("ry"), self.config, RotationParams { direction: GizmoDirection::Y }),
            RotationSubGizmo::new(self.id.with("rz"), self.config, RotationParams { direction: GizmoDirection::Z }),
            RotationSubGizmo::new(self.id.with("rs"), self.config, RotationParams { direction: GizmoDirection::View }),
        ]
    }
    fn new_translation(&self) -> [TranslationSubGizmo; 7] {
        [
            TranslationSubGizmo::new(self.id.with("txs"), self.config, TranslationParams { direction: GizmoDirection::View, transform_kind: TransformKind::Plane }),
            TranslationSubGizmo::new(self.id.with("tx"), self.config, TranslationParams { direction: GizmoDirection::X, transform_kind: TransformKind::Axis }),
            TranslationSubGizmo::new(self.id.with("ty"), self.config, TranslationParams { direction: GizmoDirection::Y, transform_kind: TransformKind::Axis }),
            TranslationSubGizmo::new(self.id.with("tz"), self.config, TranslationParams { direction: GizmoDirection::Z, transform_kind: TransformKind::Axis }),
            TranslationSubGizmo::new(self.id.with("tyz"), self.config, TranslationParams { direction: GizmoDirection::X, transform_kind: TransformKind::Plane }),
            TranslationSubGizmo::new(self.id.with("txz"), self.config, TranslationParams { direction: GizmoDirection::Y, transform_kind: TransformKind::Plane }),
            TranslationSubGizmo::new(self.id.with("txy"), self.config, TranslationParams { direction: GizmoDirection::Z, transform_kind: TransformKind::Plane }),
        ]
    }
    fn new_scale(&self) -> [ScaleSubGizmo; 7] {
        [
            ScaleSubGizmo::new(self.id.with("sxs"), self.config, ScaleParams { direction: GizmoDirection::View, transform_kind: TransformKind::Plane }),
            ScaleSubGizmo::new(self.id.with("sx"), self.config, ScaleParams { direction: GizmoDirection::X, transform_kind: TransformKind::Axis }),
            ScaleSubGizmo::new(self.id.with("sy"), self.config, ScaleParams { direction: GizmoDirection::Y, transform_kind: TransformKind::Axis }),
            ScaleSubGizmo::new(self.id.with("sz"), self.config, ScaleParams { direction: GizmoDirection::Z, transform_kind: TransformKind::Axis }),
            ScaleSubGizmo::new(self.id.with("syz"), self.config, ScaleParams { direction: GizmoDirection::X, transform_kind: TransformKind::Plane }),
            ScaleSubGizmo::new(self.id.with("sxz"), self.config, ScaleParams { direction: GizmoDirection::Y, transform_kind: TransformKind::Plane }),
            ScaleSubGizmo::new(self.id.with("sxy"), self.config, ScaleParams { direction: GizmoDirection::Z, transform_kind: TransformKind::Plane }),
        ]
    }
    fn add_subgizmos<T: SubGizmo, const N: usize>(&mut self, subgizmos: [T; N]) { for subgizmo in subgizmos { self.subgizmos.push(Box::new(subgizmo)); } }

    fn pointer_ray(&self, ui: &Ui, allow_outside_interaction: bool) -> Option<Ray> {
        let screen_pos = ui.input(|i| i.pointer.hover_pos())?;
        let interaction_rect = self.config.interaction_rect.unwrap_or(self.config.viewport);
        if !allow_outside_interaction && !interaction_rect.contains(screen_pos) {
            return None;
        }
        let mat = self.config.view_projection.inverse();
        let origin = screen_to_world(self.config.viewport, mat, screen_pos, -1.0);
        let target = screen_to_world(self.config.viewport, mat, screen_pos, 1.0);
        let direction = target.sub(origin).normalize();
        Some(Ray { screen_pos, origin, direction })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GizmoResult { pub scale: mint::Vector3<f32>, pub rotation: mint::Quaternion<f32>, pub translation: mint::Vector3<f32>, pub mode: GizmoMode, pub value: Option<[f32; 3]> }
impl GizmoResult { pub fn transform(&self) -> mint::ColumnMatrix4<f32> { Mat4::from_scale_rotation_translation(self.scale.into(), self.rotation.into(), self.translation.into()).into() } }

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GizmoMode { Rotate, Translate, Scale, Omni }
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GizmoOrientation { Global, Local }
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GizmoDirection { X, Y, Z, View }

#[derive(Debug, Copy, Clone)]
pub struct GizmoVisuals { pub x_color: Color32, pub y_color: Color32, pub z_color: Color32, pub s_color: Color32, pub inactive_alpha: f32, pub highlight_alpha: f32, pub highlight_color: Option<Color32>, pub stroke_width: f32, pub gizmo_size: f32 }
impl Default for GizmoVisuals {
    fn default() -> Self { Self { x_color: Color32::from_rgb(255, 50, 0), y_color: Color32::from_rgb(50, 255, 0), z_color: Color32::from_rgb(0, 50, 255), s_color: Color32::from_rgb(255, 255, 255), inactive_alpha: 0.5, highlight_alpha: 0.9, highlight_color: None, stroke_width: 4.0, gizmo_size: 75.0 } }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct GizmoConfig {
    pub view_matrix: DMat4,
    pub projection_matrix: DMat4,
    pub model_matrix: DMat4,
    pub viewport: Rect,
    pub interaction_rect: Option<Rect>,
    pub mode: GizmoMode,
    pub orientation: GizmoOrientation,
    pub snapping: bool,
    pub snap_angle: f32,
    pub snap_distance: f32,
    pub snap_scale: f32,
    pub visuals: GizmoVisuals,
    pub rotation: DQuat,
    pub translation: DVec3,
    pub scale: DVec3,
    pub view_projection: DMat4,
    pub mvp: DMat4,
    pub gizmo_view_forward: DVec3,
    pub scale_factor: f32,
    pub focus_distance: f32,
    pub left_handed: bool,
}
impl Default for GizmoConfig {
    fn default() -> Self { Self { view_matrix: DMat4::IDENTITY, projection_matrix: DMat4::IDENTITY, model_matrix: DMat4::IDENTITY, viewport: Rect::NOTHING, interaction_rect: None, mode: GizmoMode::Rotate, orientation: GizmoOrientation::Global, snapping: false, snap_angle: DEFAULT_SNAP_ANGLE, snap_distance: DEFAULT_SNAP_DISTANCE, snap_scale: DEFAULT_SNAP_SCALE, visuals: GizmoVisuals::default(), rotation: DQuat::IDENTITY, translation: DVec3::ZERO, scale: DVec3::ONE, view_projection: DMat4::IDENTITY, mvp: DMat4::IDENTITY, gizmo_view_forward: DVec3::ONE, scale_factor: 0.0, focus_distance: 0.0, left_handed: false } }
}
impl GizmoConfig {
    fn prepare(&mut self, ui: &Ui) {
        if self.viewport.is_negative() { self.viewport = ui.clip_rect(); }
        let (scale, rotation, translation) = self.model_matrix.to_scale_rotation_translation();
        self.rotation = rotation;
        self.translation = translation;
        self.scale = scale;
        self.view_projection = self.projection_matrix * self.view_matrix;
        self.mvp = self.projection_matrix * self.view_matrix * self.model_matrix;
        let mvp = self.mvp.to_cols_array();
        let proj = self.projection_matrix.to_cols_array();
        self.scale_factor = mvp[15] as f32 / proj[0].max(1.0e-8) as f32 / self.viewport.width().max(1.0) * 2.0;
        self.focus_distance = self.scale_factor * (self.visuals.stroke_width / 2.0 + 5.0);
        self.left_handed = if self.projection_matrix.z_axis.w == 0.0 { self.projection_matrix.z_axis.z > 0.0 } else { self.projection_matrix.z_axis.w > 0.0 };
        let gizmo_screen_pos = world_to_screen(self.viewport, self.mvp, self.translation).unwrap_or_default();
        let gizmo_view_near = screen_to_world(self.viewport, self.view_projection.inverse(), gizmo_screen_pos, -1.0);
        self.gizmo_view_forward = (gizmo_view_near - self.translation).normalize_or_zero();
    }
    pub(crate) fn view_forward(&self) -> DVec3 { self.view_matrix.row(2).xyz() }
    pub(crate) fn view_up(&self) -> DVec3 { self.view_matrix.row(1).xyz() }
    pub(crate) fn view_right(&self) -> DVec3 { self.view_matrix.row(0).xyz() }
    pub(crate) fn local_space(&self) -> bool { self.orientation == GizmoOrientation::Local || self.mode == GizmoMode::Scale }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct Ray { screen_pos: Pos2, origin: DVec3, direction: DVec3 }
#[derive(Default, Debug, Copy, Clone)]
struct GizmoState { active_subgizmo_id: Option<Id> }
pub(crate) trait WidgetData: Sized + Default + Copy + Clone + Send + Sync + 'static {
    fn load(ctx: &Context, gizmo_id: Id) -> Self { ctx.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Self>(gizmo_id)) }
    fn save(self, ctx: &Context, gizmo_id: Id) { ctx.memory_mut(|mem| mem.data.insert_temp(gizmo_id, self)); }
}
impl WidgetData for GizmoState {}
