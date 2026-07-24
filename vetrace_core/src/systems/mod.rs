//! Core systems that are generic enough to live in `vetrace_core`.
//!
//! These are not feature systems. Rendering, physics, audio, UI, networking,
//! and scripting systems live in their own crates.

pub mod hierarchy;
pub mod timer;

pub use hierarchy::{propagate_global_transforms, HierarchyPlugin};
pub use timer::{tick_timers, TimerPlugin};
