//! Vetrace Editor plugin for the active modular engine.
//!
//! The active editor implementation is split under `active_editor/`. The
//! editor transform widget uses the vendored `third_party/egui-gizmo` crate,
//! matching the map builder gizmo backend.

mod active_editor;
mod history;

pub use active_editor::*;
pub use history::UndoHistory;
