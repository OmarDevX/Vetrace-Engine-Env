pub mod engine;
pub mod component_registry;
mod component_reflection;

/// Backward-compatible path for code that still imports `engine::managers`.
#[deprecated(note = "use engine::component_registry")]
pub mod managers {
    pub use super::component_registry::*;
}

pub use component_registry::{ComponentDescriptor, ComponentManager};
pub use engine::Engine;
