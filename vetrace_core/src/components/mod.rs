//! Built-in engine components and compatibility exports.

pub mod builtins;

pub use builtins::*;

/// Compatibility path for projects that still import
/// `vetrace_core::components::builtins::*`.
#[deprecated(note = "use vetrace_core::components::builtins or vetrace_core root re-exports")]
pub mod components {
    pub use super::builtins::*;
}
