use super::engine::Engine;
use crate::events::SceneEvents;
use crate::scene::loader::{load_scene, SceneFile};
use std::collections::HashMap;

/// Manages loaded scenes and allows switching the active one at runtime.
pub struct SceneManager {
    scenes: HashMap<String, SceneFile>,
    events: HashMap<String, SceneEvents>,
    current: Option<String>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self { scenes: HashMap::new(), events: HashMap::new(), current: None }
    }

    /// Load a scene file from disk and store it under `name`.
    pub fn load_scene_from_file(&mut self, name: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let scene = load_scene(path)?;
        self.scenes.insert(name.to_string(), scene);
        self.events.entry(name.to_string()).or_insert_with(SceneEvents::new);
        Ok(())
    }

    /// Add a scene already loaded in memory.
    pub fn add_scene(&mut self, name: &str, scene: SceneFile) {
        self.scenes.insert(name.to_string(), scene);
        self.events.entry(name.to_string()).or_insert_with(SceneEvents::new);
    }

    /// Switch the active scene on the given [`Engine`].
    pub fn switch_to(&mut self, name: &str, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(scene) = self.scenes.get(name).cloned() {
            if let Some(current) = self.current.take() {
                self.events.insert(current, std::mem::take(&mut engine.scene_events));
            }
            engine.clear_scene();
            engine.load_scene(scene)?;
            engine.scene_events = self.events.remove(name).unwrap_or_else(SceneEvents::new);
            self.current = Some(name.to_string());
            Ok(())
        } else {
            Err(format!("scene '{}' not found", name).into())
        }
    }
}

