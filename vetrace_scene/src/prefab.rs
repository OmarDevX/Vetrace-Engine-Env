use glam::{Quat, Vec3};
use vetrace_core::{Actor, ActorError, Engine};

use crate::{SceneInstance, SceneNode};

/// Fluent prefab instantiation without exposing raw ECS handles.
pub struct PrefabBuilder<'a> {
    engine: &'a mut Engine,
    node: SceneNode,
    parent: Option<Actor>,
}

impl<'a> PrefabBuilder<'a> {
    pub(crate) fn new(engine: &'a mut Engine, node: &SceneNode) -> Self {
        Self { engine, node: node.clone(), parent: None }
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.node.name = name.into();
        self
    }

    pub fn at(mut self, position: Vec3) -> Self {
        self.node.transform.translation = position.to_array();
        self
    }

    pub fn rotated(mut self, rotation: Quat) -> Self {
        self.node.transform.rotation = rotation.to_array();
        self
    }

    pub fn scaled(mut self, scale: Vec3) -> Self {
        self.node.transform.scale = scale.max(Vec3::splat(0.001)).to_array();
        self
    }

    pub fn child_of(mut self, parent: Actor) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn build(self) -> Result<SceneInstance, ActorError> {
        let instance = self.node.instantiate(self.engine);
        if let (Some(parent), Some(root)) = (self.parent, instance.roots.first().copied()) {
            if let Err(error) = root.set_parent(self.engine, parent) {
                instance.unload(self.engine);
                return Err(error);
            }
        }
        Ok(instance)
    }
}

/// Scene/prefab extension surface for `Engine`.
pub trait SceneEngineExt {
    fn instantiate_prefab<'a>(&'a mut self, prefab: &SceneNode) -> PrefabBuilder<'a>;
}

impl SceneEngineExt for Engine {
    fn instantiate_prefab<'a>(&'a mut self, prefab: &SceneNode) -> PrefabBuilder<'a> {
        PrefabBuilder::new(self, prefab)
    }
}
