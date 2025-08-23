use std::collections::{HashMap, HashSet};
use crate::behaviour::component_lua::LuaComponentBehaviour;
use crate::behaviour::script::ScriptBehaviour;
use crate::ecs::Entity;
use crate::Behaviour;

/// Manages scripting, behaviors, and Lua integration
pub struct ScriptingManager {
    pub behaviours: Vec<Box<dyn Behaviour>>,
    pub script_library: HashMap<String, ScriptBehaviour>,
    pub component_behaviours: HashMap<String, LuaComponentBehaviour>,
    pub started_scripts: HashSet<Entity>,
}

impl ScriptingManager {
    pub fn new() -> Self {
        Self {
            behaviours: Vec::new(),
            script_library: HashMap::new(),
            component_behaviours: HashMap::new(),
            started_scripts: HashSet::new(),
        }
    }

    /// Add a behavior to the manager
    pub fn add_behaviour(&mut self, behaviour: Box<dyn Behaviour>) {
        self.behaviours.push(behaviour);
    }

    /// Add a script to the library
    pub fn add_script(&mut self, name: String, script: ScriptBehaviour) {
        self.script_library.insert(name, script);
    }

    /// Add a component behavior
    pub fn add_component_behaviour(&mut self, name: String, behaviour: LuaComponentBehaviour) {
        self.component_behaviours.insert(name, behaviour);
    }

    /// Mark a script as started for an entity
    pub fn mark_script_started(&mut self, entity: Entity) {
        self.started_scripts.insert(entity);
    }

    /// Check if a script is started for an entity
    pub fn is_script_started(&self, entity: Entity) -> bool {
        self.started_scripts.contains(&entity)
    }

    /// Remove started script marker for an entity
    pub fn remove_started_script(&mut self, entity: Entity) {
        self.started_scripts.remove(&entity);
    }

    /// Get script library reference
    pub fn script_library(&self) -> &HashMap<String, ScriptBehaviour> {
        &self.script_library
    }

    /// Get mutable script library reference
    pub fn script_library_mut(&mut self) -> &mut HashMap<String, ScriptBehaviour> {
        &mut self.script_library
    }

    /// Get component behaviours reference
    pub fn component_behaviours(&self) -> &HashMap<String, LuaComponentBehaviour> {
        &self.component_behaviours
    }

    /// Get mutable component behaviours reference
    pub fn component_behaviours_mut(&mut self) -> &mut HashMap<String, LuaComponentBehaviour> {
        &mut self.component_behaviours
    }

    /// Get behaviours reference
    pub fn behaviours(&self) -> &Vec<Box<dyn Behaviour>> {
        &self.behaviours
    }

    /// Get mutable behaviours reference
    pub fn behaviours_mut(&mut self) -> &mut Vec<Box<dyn Behaviour>> {
        &mut self.behaviours
    }
}
