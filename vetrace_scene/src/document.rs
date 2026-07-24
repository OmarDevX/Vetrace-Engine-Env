use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use vetrace_core::{Actor, Engine, Entity};

use crate::assets::{load_scene_render_textures, SceneTextureLoadReport};
use crate::export::export_roots_from_engine;
use crate::ids::SCENE_VERSION;
use crate::{SceneInstance, SceneNode};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneDocument {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub roots: Vec<SceneNode>,
    #[serde(default)]
    pub resources: SceneResources,
}

impl SceneDocument {
    pub fn new(name: impl Into<String>) -> Self {
        Self { version: SCENE_VERSION, name: name.into(), roots: Vec::new(), resources: SceneResources::default() }
    }

    pub fn validate(&self) -> Result<()> {
        if self.version != SCENE_VERSION {
            anyhow::bail!("unsupported scene version {} (expected {})", self.version, SCENE_VERSION);
        }
        Ok(())
    }

    pub fn to_pretty_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_engine(engine: &Engine, name: impl Into<String>) -> Self {
        let roots = export_roots_from_engine(engine);
        Self { version: SCENE_VERSION, name: name.into(), roots, resources: SceneResources::default() }
    }

    pub fn instantiate(&self, engine: &mut Engine) -> Result<SceneInstance> {
        self.validate()?;
        let mut instance = SceneInstance::default();
        for root in &self.roots {
            root.spawn_recursive_actor(engine, None, &mut instance);
        }
        vetrace_core::propagate_global_transforms(engine);
        Ok(instance)
    }

    #[deprecated(note = "use SceneDocument::instantiate")]
    pub fn spawn_into_engine(&self, engine: &mut Engine) -> Result<Vec<Entity>> {
        Ok(self.instantiate(engine)?.actors.into_iter().map(Actor::entity).collect())
    }


    pub fn instantiate_with_assets(
        &self,
        engine: &mut Engine,
        scene_path: impl AsRef<Path>,
    ) -> Result<(SceneInstance, SceneTextureLoadReport)> {
        let instance = self.instantiate(engine)?;
        let entities = instance.actors.iter().copied().map(Actor::entity).collect::<Vec<_>>();
        let texture_report = load_scene_render_textures(engine, &entities, Some(scene_path.as_ref()));
        Ok((instance, texture_report))
    }

    /// Compatibility wrapper returning raw runtime handles.
    #[deprecated(note = "use SceneDocument::instantiate_with_assets")]
    pub fn spawn_into_engine_with_assets(
        &self,
        engine: &mut Engine,
        scene_path: impl AsRef<Path>,
    ) -> Result<(Vec<Entity>, SceneTextureLoadReport)> {
        let (instance, texture_report) = self.instantiate_with_assets(engine, scene_path)?;
        Ok((instance.actors.into_iter().map(Actor::entity).collect(), texture_report))
    }

    pub fn object_count(&self) -> usize {
        fn count(node: &SceneNode) -> usize {
            1 + node.children.iter().map(count).sum::<usize>()
        }
        self.roots.iter().map(count).sum()
    }

    /// Compatibility view used by old flat prefab-map code.
    pub fn objects(&self) -> Vec<SceneNode> {
        let mut out = Vec::new();
        fn visit(node: &SceneNode, out: &mut Vec<SceneNode>) {
            let mut flat = node.clone();
            flat.children.clear();
            out.push(flat);
            for child in &node.children {
                visit(child, out);
            }
        }
        for root in &self.roots {
            visit(root, &mut out);
        }
        out
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneResources {
    #[serde(default)]
    pub source_files: Vec<String>,
}
