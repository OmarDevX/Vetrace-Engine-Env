use crate::scene::loader::SceneFile;
use std::fs;

/// Represents a reusable entity template loaded from disk.
pub struct Prefab {
    pub(crate) scene: SceneFile,
}

impl Prefab {
    /// Load a prefab JSON file located at `path`.
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let raw = fs::read_to_string(path)?;
        // Try to parse as `SceneFile` first; fallback to single `NodeFile`.
        let scene = match serde_json::from_str::<SceneFile>(&raw) {
            Ok(s) => s,
            Err(_) => {
                let node: crate::scene::loader::NodeFile = serde_json::from_str(&raw)?;
                SceneFile {
                    nodes: vec![node],
                    entities: Vec::new(),
                }
            }
        };
        Ok(Self { scene })
    }

    /// Consume the prefab and return the underlying [`SceneFile`].
    pub fn into_scene(self) -> SceneFile {
        self.scene
    }
}
