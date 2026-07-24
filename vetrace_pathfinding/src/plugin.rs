use std::any::Any;
use std::error::Error;

use vetrace_core::app::Plugin;
use vetrace_core::engine::Engine;

use crate::NavigationGrid;

#[derive(Clone, Copy, Debug)]
pub struct PathfindingSettings {
    pub cell_size: f32,
    pub agent_clearance: f32,
    pub allow_diagonal: bool,
}

impl Default for PathfindingSettings {
    fn default() -> Self {
        Self { cell_size: 1.0, agent_clearance: 0.6, allow_diagonal: true }
    }
}

#[derive(Clone, Debug, Default)]
pub struct PathfindingWorld {
    active_grid: Option<NavigationGrid>,
}

impl PathfindingWorld {
    pub fn set_active_grid(&mut self, grid: NavigationGrid) { self.active_grid = Some(grid); }
    pub fn clear(&mut self) { self.active_grid = None; }
    pub fn active_grid(&self) -> Option<&NavigationGrid> { self.active_grid.as_ref() }
    pub fn find_path(&self, start: glam::Vec3, goal: glam::Vec3) -> Option<crate::NavigationPath> {
        self.active_grid.as_ref()?.find_path(start, goal)
    }
}

pub struct PathfindingPlugin {
    settings: PathfindingSettings,
}

impl PathfindingPlugin {
    pub fn new(settings: PathfindingSettings) -> Self { Self { settings } }
    pub fn settings(&self) -> PathfindingSettings { self.settings }
}

impl Default for PathfindingPlugin {
    fn default() -> Self { Self::new(PathfindingSettings::default()) }
}

impl Plugin for PathfindingPlugin {
    fn name(&self) -> &'static str { "vetrace_pathfinding" }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource(self.settings);
        engine.insert_resource(PathfindingWorld::default());
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
