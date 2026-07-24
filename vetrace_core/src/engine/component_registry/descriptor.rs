use super::*;

/// Runtime metadata and generic operations for one registered component type.
///
/// The registry owns no Lua/editor-specific code. Scene IO, Lua, Studio,
/// undo/redo, and remote tooling all consume this same descriptor surface.
#[derive(Clone)]
pub struct ComponentDescriptor {
    pub stable_id: &'static str,
    pub display_name: &'static str,
    pub category: &'static str,
    pub rust_type_name: &'static str,
    pub type_id: TypeId,
    pub serialize: Option<SerializeComponentFn>,
    pub deserialize: Option<DeserializeComponentFn>,
    pub clone_component: Option<CloneComponentFn>,
    pub create: Option<CreateComponentFn>,
    pub remove: RemoveComponentFn,
    pub inspector: Option<InspectorFn>,
    pub schema: Option<ComponentSchema>,
    /// Whether scene/save serialization should persist this component.
    /// Reflection can remain available for transient runtime components.
    pub persistent: bool,
    pub removable: bool,
    pub lua_accessible: bool,
    aliases: Vec<&'static str>,
}

impl ComponentDescriptor {
    pub fn new<T: Component>(stable_id: &'static str, display_name: &'static str) -> Self {
        Self {
            stable_id,
            display_name,
            category: "General",
            rust_type_name: std::any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
            serialize: None,
            deserialize: None,
            clone_component: None,
            create: None,
            remove: remove_component::<T>,
            inspector: None,
            schema: None,
            persistent: false,
            removable: true,
            lua_accessible: false,
            aliases: Vec::new(),
        }
    }

    pub fn with_category(mut self, category: &'static str) -> Self {
        self.category = category;
        self
    }

    pub fn with_alias(mut self, alias: &'static str) -> Self {
        if !self.aliases.contains(&alias) { self.aliases.push(alias); }
        self
    }

    pub fn with_schema(mut self, mut schema: ComponentSchema) -> Self {
        schema.stable_id = self.stable_id.to_owned();
        schema.display_name = self.display_name.to_owned();
        schema.category = self.category.to_owned();
        schema.removable = self.removable;
        schema.lua_accessible = self.lua_accessible;
        self.schema = Some(schema);
        self
    }

    pub fn with_inspector(mut self, inspector: InspectorFn) -> Self {
        self.inspector = Some(inspector);
        self
    }

    pub fn non_removable(mut self) -> Self {
        self.removable = false;
        if let Some(schema) = self.schema.as_mut() { schema.removable = false; }
        self
    }

    pub fn hidden_from_lua(mut self) -> Self {
        self.lua_accessible = false;
        if let Some(schema) = self.schema.as_mut() { schema.lua_accessible = false; }
        self
    }

    pub fn aliases(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.aliases.iter().copied()
    }

    pub fn is_readable(&self) -> bool { self.serialize.is_some() }
    pub fn is_writable(&self) -> bool { self.deserialize.is_some() }
    pub fn is_constructible(&self) -> bool {
        self.schema
            .as_ref()
            .map(|schema| schema.constructible)
            .unwrap_or_else(|| self.create.is_some() || self.deserialize.is_some())
    }
}
