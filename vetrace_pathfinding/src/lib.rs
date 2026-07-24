//! Reusable pathfinding for Vetrace games.
//!
//! The crate owns generic navigation grids and A*. Games decide how world
//! geometry becomes blocked cells and which entities should request paths.

mod grid;
mod plugin;

pub use grid::{GridPoint, NavigationGrid, NavigationPath};
pub use plugin::{PathfindingPlugin, PathfindingSettings, PathfindingWorld};
