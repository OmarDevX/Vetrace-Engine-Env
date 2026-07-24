use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{CreateProjectRequest, RecentProject};

#[derive(Clone, Debug)]
pub enum ProjectManagerCommand {
    Open(PathBuf),
    Create(CreateProjectRequest),
    RemoveRecent(PathBuf),
    Refresh,
    Quit,
}

#[derive(Clone, Debug, Default)]
pub struct ProjectManagerSnapshot {
    pub recent: Vec<RecentProject>,
    pub status: String,
    pub busy: bool,
}

#[derive(Clone, Default)]
pub struct ProjectManagerBridge {
    pub snapshot: Arc<Mutex<ProjectManagerSnapshot>>,
    commands: Arc<Mutex<Vec<ProjectManagerCommand>>>,
}

impl ProjectManagerBridge {
    pub fn push(&self, command: ProjectManagerCommand) {
        if let Ok(mut commands) = self.commands.lock() {
            commands.push(command);
        }
    }

    pub fn drain(&self) -> Vec<ProjectManagerCommand> {
        self.commands
            .lock()
            .map(|mut commands| commands.drain(..).collect())
            .unwrap_or_default()
    }
}
