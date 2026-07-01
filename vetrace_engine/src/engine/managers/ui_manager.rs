// Note: MainWindow and SandboxWindow have been moved to vetrace_editor crate
use crate::scene::loader::SceneFile;

/// UI Manager for handling editor UI components (Legacy compatibility)
/// The actual UI management is now handled by the vetrace_editor plugin
pub struct UIManager {
    pub scene_manager: crate::engine::SceneManager,
    pub saved_scene: Option<SceneFile>,
}

impl UIManager {
    /// Create a new UI manager (legacy compatibility)
    pub fn new(scene_manager: crate::engine::SceneManager) -> Self {
        Self {
            scene_manager,
            saved_scene: None,
        }
    }

    /// Get scene manager reference
    pub fn scene_manager(&self) -> &crate::engine::SceneManager {
        &self.scene_manager
    }

    /// Get mutable scene manager reference
    pub fn scene_manager_mut(&mut self) -> &mut crate::engine::SceneManager {
        &mut self.scene_manager
    }

    /// Get saved scene reference
    pub fn saved_scene(&self) -> &Option<SceneFile> {
        &self.saved_scene
    }

    /// Get mutable saved scene reference
    pub fn saved_scene_mut(&mut self) -> &mut Option<SceneFile> {
        &mut self.saved_scene
    }
}

impl Default for UIManager {
    fn default() -> Self {
        Self::new(crate::engine::SceneManager::new())
    }
}