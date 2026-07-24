use std::collections::{BTreeSet, HashMap};

use crate::{Actor, Engine, Entity, Parent};

/// Derived hierarchy index. `Parent` remains the serialized ECS source of
/// truth; this resource provides fast parent/child traversal.
#[derive(Clone, Debug, Default)]
pub struct Hierarchy {
    parents: HashMap<Entity, Entity>,
    children: HashMap<Entity, BTreeSet<Entity>>,
}

impl Hierarchy {
    pub fn parent_of(&self, actor: Actor) -> Option<Actor> {
        self.parents.get(&actor.entity()).copied().map(Actor::from_entity)
    }

    pub fn children_of(&self, actor: Actor) -> impl Iterator<Item = Actor> + '_ {
        self.children
            .get(&actor.entity())
            .into_iter()
            .flat_map(|children| children.iter().copied())
            .map(Actor::from_entity)
    }

    pub fn child_count(&self, actor: Actor) -> usize {
        self.children.get(&actor.entity()).map(BTreeSet::len).unwrap_or(0)
    }

    pub(crate) fn set_parent(&mut self, child: Entity, parent: Option<Entity>) {
        if let Some(previous) = self.parents.remove(&child) {
            let remove_bucket = if let Some(children) = self.children.get_mut(&previous) {
                children.remove(&child);
                children.is_empty()
            } else {
                false
            };
            if remove_bucket {
                self.children.remove(&previous);
            }
        }
        if let Some(parent) = parent {
            self.parents.insert(child, parent);
            self.children.entry(parent).or_default().insert(child);
        }
    }

    pub(crate) fn remove_actor(&mut self, actor: Entity) {
        self.set_parent(actor, None);
        if let Some(children) = self.children.remove(&actor) {
            for child in children {
                self.parents.remove(&child);
            }
        }
    }

    pub(crate) fn rebuild(&mut self, pairs: impl IntoIterator<Item = (Entity, Entity)>) {
        self.parents.clear();
        self.children.clear();
        for (child, parent) in pairs { self.set_parent(child, Some(parent)); }
    }
}

impl Engine {
    pub fn hierarchy(&self) -> Option<&Hierarchy> { self.get_resource::<Hierarchy>() }

    pub(crate) fn rebuild_hierarchy_index(&mut self) {
        let pairs = self
            .world_ref()
            .query::<Parent>()
            .into_iter()
            .filter(|(_, parent)| self.world_ref().is_alive(parent.0))
            .map(|(child, parent)| (child, parent.0))
            .collect::<Vec<_>>();
        if !self.contains_resource::<Hierarchy>() { self.insert_resource(Hierarchy::default()); }
        self.get_resource_mut::<Hierarchy>().expect("hierarchy inserted").rebuild(pairs);
    }

    pub(crate) fn update_hierarchy_index(&mut self, child: Entity, parent: Option<Entity>) {
        if !self.contains_resource::<Hierarchy>() { self.insert_resource(Hierarchy::default()); }
        self.get_resource_mut::<Hierarchy>().expect("hierarchy inserted").set_parent(child, parent);
    }

    pub(crate) fn remove_from_hierarchy_index(&mut self, actor: Entity) {
        if let Some(hierarchy) = self.get_resource_mut::<Hierarchy>() { hierarchy.remove_actor(actor); }
    }
}
