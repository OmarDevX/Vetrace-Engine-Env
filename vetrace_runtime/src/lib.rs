//! Project-driven Vetrace runtime.
//!
//! This crate is the product boundary between authored projects and the
//! low-level engine subsystem crates. It validates a `VetraceProject`, installs
//! only the enabled plugins, maps project settings into engine resources, loads
//! autoload Lua scripts, instantiates the main scene, and owns lifecycle control.

mod app;
mod builder;
mod config;
mod error;
mod hot_reload;
mod input;
mod mode;
mod plugins;
mod resources;
mod runtime;
mod scene_loader;
mod scripting;
mod settings;
mod state;

pub use builder::{VetraceProjectRuntimeExt, VetraceRuntimeBuilder};
pub use config::RuntimeConfig;
pub use hot_reload::LuaProjectHotReloadPlugin;
pub use error::{RuntimeError, RuntimeResult};
pub use input::RuntimeInputMap;
pub use mode::RuntimeMode;
pub use resources::{
    ActiveRuntimeScene, RuntimeAutoloads, RuntimeCapabilities, RuntimeDiagnostics,
    RuntimeProject, RuntimeStatus,
};
pub use runtime::VetraceRuntime;
pub use state::RuntimeState;
pub use vetrace_project::VetraceProject;
