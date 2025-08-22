use std::collections::HashMap;
use std::rc::Rc;
use crate::components::generated::GeneratedSpec;
use crate::ecs::{Entity, World};
use crate::inspector::Inspectable;
use crate::scene::loader::ComponentFactory;

/// Manages component registration, factories, and behaviors
pub struct ComponentManager {
    pub component_factories: HashMap<String, ComponentFactory>,
    pub component_adders: HashMap<String, Rc<dyn Fn(&mut crate::Engine, Entity)>>,
    pub component_removers: HashMap<String, Rc<dyn Fn(&mut crate::Engine, Entity)>>,
    pub component_editors: HashMap<String, Rc<dyn Fn(&mut crate::Engine, Entity, &mut egui::Ui)>>,
    pub component_checkers: HashMap<String, Rc<dyn Fn(&World, Entity) -> bool>>,
    pub component_accessors: HashMap<String, fn(&mut crate::Engine, Entity) -> Option<&mut dyn Inspectable>>,
    pub generated_components: Vec<String>,
    pub generated_specs: HashMap<String, GeneratedSpec>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            component_factories: HashMap::new(),
            component_adders: HashMap::new(),
            component_removers: HashMap::new(),
            component_editors: HashMap::new(),
            component_checkers: HashMap::new(),
            component_accessors: HashMap::new(),
            generated_components: Vec::new(),
            generated_specs: HashMap::new(),
        }
    }

    /// Register a component factory
    pub fn register_factory(&mut self, name: String, factory: ComponentFactory) {
        self.component_factories.insert(name, factory);
    }

    /// Register a component adder function
    pub fn register_adder(&mut self, name: String, adder: Rc<dyn Fn(&mut crate::Engine, Entity)>) {
        self.component_adders.insert(name, adder);
    }

    /// Register a component remover function
    pub fn register_remover(&mut self, name: String, remover: Rc<dyn Fn(&mut crate::Engine, Entity)>) {
        self.component_removers.insert(name, remover);
    }

    /// Register a component editor function
    pub fn register_editor(&mut self, name: String, editor: Rc<dyn Fn(&mut crate::Engine, Entity, &mut egui::Ui)>) {
        self.component_editors.insert(name, editor);
    }

    /// Register a component checker function
    pub fn register_checker(&mut self, name: String, checker: Rc<dyn Fn(&World, Entity) -> bool>) {
        self.component_checkers.insert(name, checker);
    }

    /// Register a component accessor function
    pub fn register_accessor(&mut self, name: String, accessor: fn(&mut crate::Engine, Entity) -> Option<&mut dyn Inspectable>) {
        self.component_accessors.insert(name, accessor);
    }

    /// Add a generated component
    pub fn add_generated_component(&mut self, name: String) {
        self.generated_components.push(name);
    }

    /// Register a generated spec
    pub fn register_generated_spec(&mut self, name: String, spec: GeneratedSpec) {
        self.generated_specs.insert(name, spec);
    }

    /// Get component factory
    pub fn get_factory(&self, name: &str) -> Option<&ComponentFactory> {
        self.component_factories.get(name)
    }

    /// Get component adder
    pub fn get_adder(&self, name: &str) -> Option<&Rc<dyn Fn(&mut crate::Engine, Entity)>> {
        self.component_adders.get(name)
    }

    /// Get component remover
    pub fn get_remover(&self, name: &str) -> Option<&Rc<dyn Fn(&mut crate::Engine, Entity)>> {
        self.component_removers.get(name)
    }

    /// Get component editor
    pub fn get_editor(&self, name: &str) -> Option<&Rc<dyn Fn(&mut crate::Engine, Entity, &mut egui::Ui)>> {
        self.component_editors.get(name)
    }

    /// Get component checker
    pub fn get_checker(&self, name: &str) -> Option<&Rc<dyn Fn(&World, Entity) -> bool>> {
        self.component_checkers.get(name)
    }

    /// Get component accessor
    pub fn get_accessor(&self, name: &str) -> Option<&fn(&mut crate::Engine, Entity) -> Option<&mut dyn Inspectable>> {
        self.component_accessors.get(name)
    }

    /// Get generated spec
    pub fn get_generated_spec(&self, name: &str) -> Option<&GeneratedSpec> {
        self.generated_specs.get(name)
    }

    /// Get all generated component names
    pub fn generated_components(&self) -> &Vec<String> {
        &self.generated_components
    }
}
