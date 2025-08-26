use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use crate::engine::engine::Engine;
use crate::ecs::Entity;

/// Component factories now receive a JSON value with the component parameters
pub type ComponentFactory = fn(Entity, &mut Engine, &Value);

/// Generic representation of a component inside a scene file
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ComponentFile {
    pub name: String,
    #[serde(default)]
    pub data: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeFile {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub size: [f32; 3],
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
    pub is_cube: bool,
    pub components: Vec<ComponentFile>,
}

fn default_scale() -> [f32; 3] {
    [1.0; 3]
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct EntityFile {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub components: Vec<ComponentFile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SceneFile {
    pub nodes: Vec<NodeFile>,
    #[serde(default)]
    pub entities: Vec<EntityFile>,
}

/// Just load and parse scene JSON file, return `SceneFile`
/// Don't do any game logic or component insertion here!
pub fn load_scene(path: &str) -> Result<SceneFile, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    let scene_file: SceneFile = serde_json::from_str(&raw)?;
    Ok(scene_file)
}

/// Save a `SceneFile` to JSON on disk
pub fn save_scene(path: &str, scene: &SceneFile) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(scene)?;
    fs::write(path, json)?;
    Ok(())
}
