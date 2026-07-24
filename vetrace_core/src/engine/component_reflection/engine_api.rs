use super::*;

impl Engine {
    /// Serialize every registered component that opted into persistence.
    pub fn serialize_registered_components(
        &self,
        actor: Actor,
    ) -> BTreeMap<String, serde_json::Value> {
        let descriptors = self
            .get_resource::<ComponentManager>()
            .map(|registry| registry.descriptors().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        descriptors
            .into_iter()
            .filter(|descriptor| descriptor.persistent)
            .filter_map(|descriptor| {
                let serialize = descriptor.serialize?;
                let value = DynamicValue::from_json(serialize(self, actor)?);
                let value = descriptor
                    .schema
                    .as_ref()
                    .and_then(|schema| filter_component_value(&value, schema, FieldVisibility::Serializable))
                    .unwrap_or(value);
                Some((descriptor.stable_id.to_string(), value.into_json()))
            })
            .collect()
    }

    /// Resolve any stable ID, Rust type name, display name, or registered alias
    /// to the canonical namespaced component ID.
    pub fn resolve_component_id(&self, identifier: &str) -> Option<&'static str> {
        self.get_resource::<ComponentManager>()?.resolve_id(identifier)
    }

    /// Returns canonical IDs for reflected/readable components currently
    /// present on an actor.
    pub fn registered_components(&self, actor: Actor) -> Vec<&'static str> {
        let Some(registry) = self.get_resource::<ComponentManager>() else { return Vec::new(); };
        registry
            .descriptors()
            .filter_map(|descriptor| {
                let serialize = descriptor.serialize?;
                serialize(self, actor).is_some().then_some(descriptor.stable_id)
            })
            .collect()
    }

    /// Returns canonical IDs for components currently present and exposed to
    /// Lua. This is used by generic scripting enumeration.
    pub fn lua_components(&self, actor: Actor) -> Vec<&'static str> {
        let Some(registry) = self.get_resource::<ComponentManager>() else { return Vec::new(); };
        registry
            .descriptors()
            .filter(|descriptor| descriptor.lua_accessible)
            .filter_map(|descriptor| {
                let serialize = descriptor.serialize?;
                serialize(self, actor).is_some().then_some(descriptor.stable_id)
            })
            .collect()
    }

    pub fn registered_component_value(
        &self,
        actor: Actor,
        identifier: &str,
    ) -> Result<DynamicValue, ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        let serialize = descriptor
            .serialize
            .ok_or_else(|| ReflectionError::ComponentNotReadable(descriptor.stable_id.to_owned()))?;
        let value = serialize(self, actor)
            .ok_or_else(|| ReflectionError::ComponentNotPresent(descriptor.stable_id.to_owned()))?;
        Ok(DynamicValue::from_json(value))
    }

    pub fn registered_component_field(
        &self,
        actor: Actor,
        identifier: &str,
        path: &FieldPath,
    ) -> Result<DynamicValue, ReflectionError> {
        self.registered_component_value(actor, identifier)?.get(path).cloned()
    }

    pub fn set_registered_component_value(
        &mut self,
        actor: Actor,
        identifier: &str,
        value: DynamicValue,
    ) -> Result<(), ReflectionError> {
        self.set_registered_component_value_with_access(actor, identifier, value, UpdateAccess::Generic)
    }

    pub fn set_lua_component_value(
        &mut self,
        actor: Actor,
        identifier: &str,
        value: DynamicValue,
    ) -> Result<(), ReflectionError> {
        self.set_registered_component_value_with_access(actor, identifier, value, UpdateAccess::Lua)
    }

    fn set_registered_component_value_with_access(
        &mut self,
        actor: Actor,
        identifier: &str,
        value: DynamicValue,
        access: UpdateAccess,
    ) -> Result<(), ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        let deserialize = descriptor
            .deserialize
            .ok_or_else(|| ReflectionError::ComponentReadOnly(descriptor.stable_id.to_owned()))?;
        let value = if let Some(schema) = descriptor.schema.as_ref() {
            let mut current = self.registered_component_value(actor, descriptor.stable_id)?;
            merge_component_update(&mut current, value, schema, access)?;
            current
        } else {
            value
        };
        deserialize(self, actor, value.into_json()).map_err(ReflectionError::Operation)
    }

    pub fn set_registered_component_field(
        &mut self,
        actor: Actor,
        identifier: &str,
        path: &FieldPath,
        value: DynamicValue,
    ) -> Result<(), ReflectionError> {
        self.set_registered_component_field_with_access(
            actor,
            identifier,
            path,
            value,
            UpdateAccess::Generic,
        )
    }

    pub fn set_lua_component_field(
        &mut self,
        actor: Actor,
        identifier: &str,
        path: &FieldPath,
        value: DynamicValue,
    ) -> Result<(), ReflectionError> {
        self.set_registered_component_field_with_access(
            actor,
            identifier,
            path,
            value,
            UpdateAccess::Lua,
        )
    }

    fn set_registered_component_field_with_access(
        &mut self,
        actor: Actor,
        identifier: &str,
        path: &FieldPath,
        value: DynamicValue,
        access: UpdateAccess,
    ) -> Result<(), ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        let deserialize = descriptor
            .deserialize
            .ok_or_else(|| ReflectionError::ComponentReadOnly(descriptor.stable_id.to_owned()))?;
        let mut component = self.registered_component_value(actor, descriptor.stable_id)?;
        if let Some(schema) = descriptor.schema.as_ref() {
            merge_component_field_update(&mut component, path, value, schema, access)?;
        } else {
            component.set(path, value)?;
        }
        deserialize(self, actor, component.into_json()).map_err(ReflectionError::Operation)
    }

    pub fn lua_component_field(
        &self,
        actor: Actor,
        identifier: &str,
        path: &FieldPath,
    ) -> Result<DynamicValue, ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        if !descriptor.lua_accessible {
            return Err(ReflectionError::LuaAccessDenied(descriptor.stable_id.to_owned()));
        }
        let value = self.registered_component_value(actor, descriptor.stable_id)?;
        if path.is_root() {
            return match descriptor.schema.as_ref() {
                Some(schema) => filter_component_value(&value, schema, FieldVisibility::Lua)
                    .ok_or_else(|| ReflectionError::LuaAccessDenied(descriptor.stable_id.to_owned())),
                None => Ok(value),
            };
        }
        if let Some(schema) = descriptor.schema.as_ref() {
            let field = schema_field(schema, path)
                .ok_or_else(|| ReflectionError::MissingField(path.to_string()))?;
            ensure_field_access(field, descriptor.stable_id, path, UpdateAccess::Lua, false)?;
        }
        value.get(path).cloned()
    }

    pub fn add_registered_component(
        &mut self,
        actor: Actor,
        identifier: &str,
        value: Option<DynamicValue>,
    ) -> Result<(), ReflectionError> {
        self.add_registered_component_with_access(actor, identifier, value, UpdateAccess::Generic)
    }

    pub fn add_lua_component(
        &mut self,
        actor: Actor,
        identifier: &str,
        value: Option<DynamicValue>,
    ) -> Result<(), ReflectionError> {
        self.add_registered_component_with_access(actor, identifier, value, UpdateAccess::Lua)
    }

    fn add_registered_component_with_access(
        &mut self,
        actor: Actor,
        identifier: &str,
        value: Option<DynamicValue>,
        access: UpdateAccess,
    ) -> Result<(), ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        if matches!(access, UpdateAccess::Lua) && !descriptor.lua_accessible {
            return Err(ReflectionError::LuaAccessDenied(descriptor.stable_id.to_owned()));
        }
        if self.registered_component_value(actor, descriptor.stable_id).is_ok() {
            return match value {
                Some(value) => self.set_registered_component_value_with_access(
                    actor,
                    descriptor.stable_id,
                    value,
                    access,
                ),
                None => Ok(()),
            };
        }
        if !descriptor.is_constructible() {
            return Err(ReflectionError::ComponentNotConstructible(
                descriptor.stable_id.to_owned(),
            ));
        }
        let value = match (value, descriptor.schema.as_ref()) {
            (Some(value), Some(schema)) => {
                let mut defaults = component_default_value(schema);
                merge_component_update(&mut defaults, value, schema, access)?;
                Some(defaults)
            }
            (value, _) => value,
        };
        if let Some(create) = descriptor.create {
            return create(self, actor, value).map_err(ReflectionError::Operation);
        }
        if let (Some(value), Some(deserialize)) = (value, descriptor.deserialize) {
            return deserialize(self, actor, value.into_json()).map_err(ReflectionError::Operation);
        }
        Err(ReflectionError::ComponentNotConstructible(
            descriptor.stable_id.to_owned(),
        ))
    }

    /// Schemas for every reflected/readable registered component. Studio uses
    /// this for add-component dialogs without importing subsystem component
    /// types or maintaining a central hardcoded list.
    pub fn registered_component_schemas(&self) -> Vec<ComponentSchema> {
        let Some(registry) = self.get_resource::<ComponentManager>() else { return Vec::new(); };
        registry
            .descriptors()
            .filter(|descriptor| descriptor.is_readable())
            .map(|descriptor| {
                descriptor.schema.clone().unwrap_or_else(|| {
                    let mut schema = ComponentSchema::new(
                        descriptor.stable_id,
                        descriptor.display_name,
                        descriptor.category,
                    );
                    schema.constructible = descriptor.is_constructible();
                    schema.removable = descriptor.removable;
                    schema.serializable = descriptor.persistent;
                    schema.lua_accessible = descriptor.lua_accessible;
                    schema
                })
            })
            .collect()
    }

    /// Schemas for reflected components currently present on an actor.
    pub fn actor_component_schemas(&self, actor: Actor) -> Vec<ComponentSchema> {
        self.registered_components(actor)
            .into_iter()
            .filter_map(|id| self.component_schema(Some(actor), id).ok())
            .collect()
    }

    pub fn component_schema(
        &self,
        actor: Option<Actor>,
        identifier: &str,
    ) -> Result<ComponentSchema, ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        if let Some(schema) = descriptor.schema {
            return Ok(schema);
        }
        let value = match actor {
            Some(actor) => self.registered_component_value(actor, descriptor.stable_id)?,
            None => DynamicValue::Object(BTreeMap::new()),
        };
        let mut schema = ComponentSchema::inferred(
            descriptor.stable_id,
            descriptor.display_name,
            descriptor.category,
            value,
        );
        schema.constructible = descriptor.is_constructible();
        schema.removable = descriptor.removable;
        schema.serializable = descriptor.persistent;
        schema.lua_accessible = descriptor.lua_accessible;
        Ok(schema)
    }

    pub fn ensure_lua_component_access(
        &self,
        identifier: &str,
        path: Option<&FieldPath>,
    ) -> Result<&'static str, ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        if !descriptor.lua_accessible {
            return Err(ReflectionError::LuaAccessDenied(descriptor.stable_id.to_owned()));
        }
        if let (Some(schema), Some(path)) = (descriptor.schema.as_ref(), path) {
            if !path.is_root() {
                let field = schema_field(schema, path)
                    .ok_or_else(|| ReflectionError::MissingField(path.to_string()))?;
                if !field.lua_accessible {
                    return Err(ReflectionError::LuaAccessDenied(format!(
                        "{}.{}",
                        descriptor.stable_id,
                        path
                    )));
                }
            }
        }
        Ok(descriptor.stable_id)
    }

    /// Apply one component by its stable registry ID.
    pub fn apply_registered_component(
        &mut self,
        actor: Actor,
        stable_id: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(stable_id))
            .cloned()
            .ok_or_else(|| format!("component `{stable_id}` is not registered"))?;
        if let Some(create) = descriptor.create {
            return create(self, actor, Some(DynamicValue::from_json(value)));
        }
        let deserialize = descriptor
            .deserialize
            .ok_or_else(|| format!("component `{stable_id}` is not deserializable"))?;
        deserialize(self, actor, value)
    }

    /// Clone all clone-enabled registered components from one actor to another.
    pub fn clone_registered_components(&mut self, source: Actor, target: Actor) -> Vec<String> {
        let descriptors = self
            .get_resource::<ComponentManager>()
            .map(|registry| registry.descriptors().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut failures = Vec::new();
        for descriptor in descriptors {
            let Some(clone_component) = descriptor.clone_component else { continue; };
            if let Err(error) = clone_component(self, source, target) {
                failures.push(format!("{}: {error}", descriptor.stable_id));
            }
        }
        failures
    }

    pub fn remove_registered_component(&mut self, actor: Actor, stable_id: &str) -> Result<bool, String> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(stable_id))
            .cloned()
            .ok_or_else(|| format!("component `{stable_id}` is not registered"))?;
        if !descriptor.removable {
            return Err(format!("component `{}` is not removable", descriptor.stable_id));
        }
        Ok((descriptor.remove)(self, actor))
    }

    pub fn remove_reflected_component(
        &mut self,
        actor: Actor,
        identifier: &str,
    ) -> Result<bool, ReflectionError> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(identifier))
            .cloned()
            .ok_or_else(|| ReflectionError::ComponentNotRegistered(identifier.to_owned()))?;
        if !descriptor.removable {
            return Err(ReflectionError::ComponentNotRemovable(descriptor.stable_id.to_owned()));
        }
        Ok((descriptor.remove)(self, actor))
    }

    pub fn inspect_registered_component(&mut self, actor: Actor, stable_id: &str) -> Result<bool, String> {
        let descriptor = self
            .get_resource::<ComponentManager>()
            .and_then(|registry| registry.descriptor(stable_id))
            .cloned()
            .ok_or_else(|| format!("component `{stable_id}` is not registered"))?;
        let inspector = descriptor
            .inspector
            .ok_or_else(|| format!("component `{stable_id}` has no inspector"))?;
        Ok(inspector(self, actor))
    }
}
