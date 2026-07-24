use super::*;

/// Component registry used by scene IO, generic reflection, editor inspection,
/// cloning, scripting, and plugins.
#[derive(Default)]
pub struct ComponentManager {
    by_stable_id: BTreeMap<&'static str, ComponentDescriptor>,
    by_type: BTreeMap<&'static str, &'static str>,
    by_alias: BTreeMap<String, &'static str>,
}

impl ComponentManager {
    pub fn new() -> Self { Self::default() }

    /// Compatibility registration for runtime-only or opaque components.
    /// Such components are discoverable but are not automatically serialized
    /// or exposed to Lua.
    pub fn register<T: Component>(&mut self) {
        let type_name = std::any::type_name::<T>();
        self.register_descriptor(ComponentDescriptor::new::<T>(type_name, short_type_name(type_name)));
    }

    pub fn register_named<T: Component>(&mut self, stable_id: &'static str, display_name: &'static str) {
        self.register_descriptor(ComponentDescriptor::new::<T>(stable_id, display_name));
    }

    /// Generic serde-backed registration. Every component registered here is
    /// automatically readable/writable through reflection and Lua without a
    /// type-specific binding. It is not default-constructible unless values are
    /// provided by the caller.
    pub fn register_serializable<T>(
        &mut self,
        stable_id: &'static str,
        display_name: &'static str,
    ) where
        T: Component + Clone + Serialize + DeserializeOwned,
    {
        let mut descriptor = ComponentDescriptor::new::<T>(stable_id, display_name);
        descriptor.serialize = Some(serialize_component::<T>);
        descriptor.deserialize = Some(deserialize_component::<T>);
        descriptor.clone_component = Some(clone_component::<T>);
        descriptor.persistent = true;
        descriptor.lua_accessible = true;
        self.register_descriptor(descriptor);
    }

    pub fn register_serializable_no_clone<T>(
        &mut self,
        stable_id: &'static str,
        display_name: &'static str,
    ) where
        T: Component + Serialize + DeserializeOwned,
    {
        let mut descriptor = ComponentDescriptor::new::<T>(stable_id, display_name);
        descriptor.serialize = Some(serialize_component::<T>);
        descriptor.deserialize = Some(deserialize_component::<T>);
        descriptor.persistent = true;
        descriptor.lua_accessible = true;
        self.register_descriptor(descriptor);
    }

    /// Register a persistable component whose value is managed by a dedicated
    /// API and therefore must not be generically deserialized, cloned, removed,
    /// or changed from Lua.
    pub fn register_serializable_readonly<T>(
        &mut self,
        stable_id: &'static str,
        display_name: &'static str,
    ) where
        T: Component + Serialize,
    {
        let mut descriptor = ComponentDescriptor::new::<T>(stable_id, display_name);
        descriptor.serialize = Some(serialize_component::<T>);
        descriptor.persistent = true;
        descriptor.removable = false;
        descriptor.lua_accessible = true;
        self.register_descriptor(descriptor);
    }

    /// Register a fully reflected component using explicit derive/manual
    /// metadata. This is the preferred API for custom gameplay components.
    pub fn register_reflected<T>(&mut self)
    where
        T: VetraceComponent,
    {
        let schema = T::component_schema();
        let mut descriptor = reflected_descriptor::<T>(
            T::STABLE_ID,
            T::DISPLAY_NAME,
            T::CATEGORY,
        );
        descriptor.removable = schema.removable;
        descriptor.lua_accessible = schema.lua_accessible;
        descriptor.persistent = schema.serializable;
        if !schema.constructible { descriptor.create = None; }
        descriptor.schema = Some(schema);
        self.register_descriptor(descriptor);
    }

    /// Register a fully reflected component without a derive. The schema is
    /// inferred recursively from `T::default()` and may later be customized by
    /// registering a descriptor directly.
    pub fn register_reflected_named<T>(
        &mut self,
        stable_id: &'static str,
        display_name: &'static str,
        category: &'static str,
    ) where
        T: Component + Clone + Default + Serialize + DeserializeOwned,
    {
        let mut descriptor = reflected_descriptor::<T>(stable_id, display_name, category);
        let default_value = DynamicValue::from_serialize(&T::default()).unwrap_or_default();
        descriptor.schema = Some(ComponentSchema::inferred(
            stable_id,
            display_name,
            category,
            default_value,
        ));
        self.register_descriptor(descriptor);
    }

    /// Fully reflected but excluded from authored scene/save persistence.
    /// Useful for renderer/physics runtime state that should still be generic in
    /// Lua and diagnostics.
    pub fn register_reflected_transient_named<T>(
        &mut self,
        stable_id: &'static str,
        display_name: &'static str,
        category: &'static str,
    ) where
        T: Component + Clone + Default + Serialize + DeserializeOwned,
    {
        let mut descriptor = reflected_descriptor::<T>(stable_id, display_name, category);
        descriptor.persistent = false;
        let default_value = DynamicValue::from_serialize(&T::default()).unwrap_or_default();
        let mut schema = ComponentSchema::inferred(stable_id, display_name, category, default_value);
        schema.serializable = false;
        descriptor.schema = Some(schema);
        self.register_descriptor(descriptor);
    }

    /// Read-only reflected runtime state excluded from scene/save persistence.
    pub fn register_serializable_readonly_transient<T>(
        &mut self,
        stable_id: &'static str,
        display_name: &'static str,
    ) where
        T: Component + Serialize,
    {
        let mut descriptor = ComponentDescriptor::new::<T>(stable_id, display_name);
        descriptor.serialize = Some(serialize_component::<T>);
        descriptor.persistent = false;
        descriptor.removable = false;
        descriptor.lua_accessible = true;
        self.register_descriptor(descriptor);
    }

    pub fn register_descriptor(&mut self, mut descriptor: ComponentDescriptor) {
        let constructible = descriptor.is_constructible();
        let serializable = descriptor.persistent;
        if let Some(schema) = descriptor.schema.as_mut() {
            schema.stable_id = descriptor.stable_id.to_owned();
            schema.display_name = descriptor.display_name.to_owned();
            schema.category = descriptor.category.to_owned();
            schema.removable = descriptor.removable;
            schema.lua_accessible = descriptor.lua_accessible;
            schema.constructible = constructible;
            schema.serializable = serializable;
        }

        let stable_id = descriptor.stable_id;
        self.by_type.insert(descriptor.rust_type_name, stable_id);
        self.insert_alias(stable_id, stable_id);
        self.insert_alias(descriptor.display_name, stable_id);
        self.insert_alias(descriptor.rust_type_name, stable_id);
        self.insert_alias(short_type_name(descriptor.rust_type_name), stable_id);
        for alias in descriptor.aliases() { self.insert_alias(alias, stable_id); }
        self.by_stable_id.insert(stable_id, descriptor);
    }

    pub fn register_alias(&mut self, stable_id: &'static str, alias: &'static str) -> Result<(), String> {
        if !self.by_stable_id.contains_key(stable_id) {
            return Err(format!("component `{stable_id}` is not registered"));
        }
        self.insert_alias(alias, stable_id);
        Ok(())
    }

    /// Publish the serialized variants for one reflected enum field.
    ///
    /// The owning component/plugin registers this metadata once; Studio, Lua,
    /// scene tooling, and future remote inspectors consume it generically.
    pub fn register_enum_field<E: VetraceEnum>(
        &mut self,
        identifier: &str,
        path: &str,
    ) -> Result<(), String> {
        let stable_id = self
            .resolve_id(identifier)
            .ok_or_else(|| format!("component `{identifier}` is not registered"))?;
        let descriptor = self
            .by_stable_id
            .get_mut(stable_id)
            .ok_or_else(|| format!("component `{identifier}` is not registered"))?;
        let schema = descriptor
            .schema
            .as_mut()
            .ok_or_else(|| format!("component `{identifier}` does not publish a reflection schema"))?;
        let path = FieldPath::parse(path).map_err(|error| error.to_string())?;
        let field = schema_field_mut(&mut schema.fields, path.segments())
            .ok_or_else(|| format!("component `{identifier}` has no reflected field `{path}`"))?;
        field.kind = FieldKind::Enum;
        field.enum_variants = E::variants().iter().map(|variant| (*variant).to_owned()).collect();
        Ok(())
    }

    /// Component-owned edit policy used by Studio and generic tooling.
    ///
    /// This keeps non-removable rules in the registering subsystem instead of
    /// hardcoding component names in editor or Lua crates.
    pub fn set_removable(&mut self, identifier: &str, removable: bool) -> Result<(), String> {
        let stable_id = self
            .resolve_id(identifier)
            .ok_or_else(|| format!("component `{identifier}` is not registered"))?;
        let descriptor = self
            .by_stable_id
            .get_mut(stable_id)
            .ok_or_else(|| format!("component `{identifier}` is not registered"))?;
        descriptor.removable = removable;
        if let Some(schema) = descriptor.schema.as_mut() {
            schema.removable = removable;
        }
        Ok(())
    }

    fn insert_alias(&mut self, alias: &str, stable_id: &'static str) {
        self.by_alias.insert(alias.to_owned(), stable_id);
        self.by_alias.insert(normalize_alias(alias), stable_id);
    }

    /// Compatibility entry point for old plugins that only had a string.
    pub fn register_name(&mut self, name: &'static str) {
        self.by_type.insert(name, name);
        self.insert_alias(name, name);
    }

    pub fn contains<T: Component>(&self) -> bool { self.by_type.contains_key(std::any::type_name::<T>()) }
    pub fn contains_name(&self, name: &str) -> bool { self.resolve_id(name).is_some() }

    pub fn resolve_id(&self, identifier: &str) -> Option<&'static str> {
        if self.by_stable_id.contains_key(identifier) {
            return self.by_stable_id.get_key_value(identifier).map(|(id, _)| *id);
        }
        self.by_alias
            .get(identifier)
            .copied()
            .or_else(|| self.by_alias.get(&normalize_alias(identifier)).copied())
            .or_else(|| self.by_type.get(identifier).copied())
    }

    pub fn descriptor(&self, stable_id: &str) -> Option<&ComponentDescriptor> {
        self.resolve_id(stable_id).and_then(|resolved| self.by_stable_id.get(resolved))
    }

    pub fn descriptor_for<T: Component>(&self) -> Option<&ComponentDescriptor> {
        let stable_id = self.by_type.get(std::any::type_name::<T>())?;
        self.by_stable_id.get(stable_id)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = &ComponentDescriptor> { self.by_stable_id.values() }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.by_stable_id.keys().copied().chain(
            self.by_type
                .keys()
                .copied()
                .filter(|name| !self.by_stable_id.contains_key(name)),
        )
    }
}
