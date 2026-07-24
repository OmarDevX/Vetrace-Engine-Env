//! Generic Vetrace project player.
//!
//! This crate intentionally contains no game-specific systems. Native Rust
//! games remain free to use the subsystem crates directly, while project-driven
//! games can be launched through this thin executable and `vetrace_runtime`.

mod args;
mod error;
mod player;

pub use args::{CliError, HELP, ParseOutcome, PlayerArgs, parse_env, parse_from};
pub use error::{
    EXIT_PROJECT, EXIT_RUNTIME_EXECUTION, EXIT_RUNTIME_SETUP, EXIT_SUCCESS, EXIT_USAGE,
    PlayerError,
};
pub use player::{run, write_error_diagnostic, write_project_info};
