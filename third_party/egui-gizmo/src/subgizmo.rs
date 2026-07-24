use std::hash::Hash;
use std::ops::Deref;

use egui::{Id, Ui};

use crate::{GizmoConfig, GizmoResult, Ray};

pub(crate) use arcball::ArcballSubGizmo;
pub(crate) use rotation::RotationSubGizmo;
pub(crate) use scale::ScaleSubGizmo;
pub(crate) use translation::TranslationSubGizmo;

pub(crate) mod arcball;
pub(crate) mod common;
pub(crate) mod rotation;
pub(crate) mod scale;
pub(crate) mod translation;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum TransformKind { Axis, Plane }

pub(crate) trait SubGizmoKind: 'static { type Params; type State: Copy + Clone + Send + Sync + Default + 'static; }

pub(crate) struct SubGizmoConfig<T: SubGizmoKind> {
    id: Id,
    pub(crate) config: GizmoConfig,
    pub(crate) focused: bool,
    pub(crate) active: bool,
    pub(crate) opacity: f32,
    params: T::Params,
}

impl<T: SubGizmoKind> Deref for SubGizmoConfig<T> {
    type Target = T::Params;
    fn deref(&self) -> &Self::Target { &self.params }
}

pub(crate) trait SubGizmoBase: 'static {
    fn id(&self) -> Id;
    fn set_focused(&mut self, focused: bool);
    fn set_active(&mut self, active: bool);
    fn is_active(&self) -> bool;
}

impl<T: SubGizmoKind> SubGizmoBase for SubGizmoConfig<T> {
    fn id(&self) -> Id { self.id }
    fn set_focused(&mut self, focused: bool) { self.focused = focused; }
    fn set_active(&mut self, active: bool) { self.active = active; }
    fn is_active(&self) -> bool { self.active }
}

pub(crate) trait SubGizmo: SubGizmoBase {
    fn pick(&mut self, ui: &Ui, ray: Ray) -> Option<f64>;
    fn update(&mut self, ui: &Ui, ray: Ray) -> Option<GizmoResult>;
    fn draw(&mut self, ui: &Ui);
}

impl<T> SubGizmoConfig<T>
where T: SubGizmoKind {
    pub fn new(id_source: impl Hash, config: GizmoConfig, params: T::Params) -> Self {
        Self { id: Id::new(id_source), config, focused: false, active: false, opacity: 0.0, params }
    }
    pub fn state(&self, ui: &Ui) -> T::State {
        ui.ctx().memory_mut(|mem| *mem.data.get_temp_mut_or_default::<T::State>(self.id))
    }
    pub fn update_state_with(&self, ui: &Ui, fun: impl FnOnce(&mut T::State)) {
        let mut state = self.state(ui);
        fun(&mut state);
        ui.ctx().memory_mut(|mem| mem.data.insert_temp(self.id, state));
    }
}
