use std::any::Any;
use std::error::Error;

use vetrace_core::app::Plugin;
use vetrace_core::backends::ScriptingBackend;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::Stage;

use crate::backend::{fixed_update_scripts, load_scripts_from_dir, LuaScriptingBackend};
use crate::components::ScriptComponent;
use crate::diagnostics::{LuaDiagnostics, LuaRuntimeConfig};
use crate::input::LuaInputMap;
use crate::state::LuaScriptingState;
use crate::runtime_api::{LuaAudioSettings, LuaNetworkState};

pub struct LuaScriptingPlugin {
    auto_load_dir: Option<std::path::PathBuf>,
    execute_scripts: bool,
}

impl LuaScriptingPlugin {
    pub fn new() -> Self {
        Self { auto_load_dir: None, execute_scripts: true }
    }

    /// Installs Lua reflection, component registration, diagnostics, and editor
    /// tooling without starting or updating authored gameplay scripts.
    pub fn authoring_only() -> Self {
        Self { auto_load_dir: None, execute_scripts: false }
    }

    pub fn with_auto_load_dir(path: impl Into<std::path::PathBuf>) -> Self {
        Self { auto_load_dir: Some(path.into()), execute_scripts: true }
    }

    pub fn with_execution_enabled(mut self, enabled: bool) -> Self {
        self.execute_scripts = enabled;
        self
    }
}

impl Default for LuaScriptingPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for LuaScriptingPlugin {
    fn name(&self) -> &'static str { "lua_scripting" }
    fn update_stage(&self) -> Stage { Stage::Update }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource::<Box<dyn ScriptingBackend>>(Box::new(LuaScriptingBackend::new()));
        // Product players may pre-create the Lua state to install the external
        // debugger before RuntimeApp starts autoloads and entity `ready`
        // callbacks. Preserve that state instead of replacing its VM/hook.
        if !engine.contains_resource::<LuaScriptingState>() {
            engine.insert_resource(LuaScriptingState::new());
        }
        if !engine.contains_resource::<LuaDiagnostics>() {
            engine.insert_resource(LuaDiagnostics::default());
        }
        if !engine.contains_resource::<LuaRuntimeConfig>() {
            engine.insert_resource(LuaRuntimeConfig::default());
        }
        if !engine.contains_resource::<LuaInputMap>() {
            engine.insert_resource(LuaInputMap::default());
        }
        if !engine.contains_resource::<LuaNetworkState>() {
            engine.insert_resource(LuaNetworkState::default());
        }
        if !engine.contains_resource::<LuaAudioSettings>() {
            engine.insert_resource(LuaAudioSettings::default());
        }
        if let Some(manager) = engine.get_resource_mut::<ComponentManager>() {
            manager.register_reflected::<ScriptComponent>();
        }

        let execute_scripts = self.execute_scripts;
        engine.add_system(Stage::FixedUpdate, "lua.fixed_update", move |engine, dt| {
            if execute_scripts {
                fixed_update_scripts(engine, dt);
            }
        });
        if self.execute_scripts {
            if let Some(dir) = &self.auto_load_dir {
                load_scripts_from_dir(engine, dir)?;
            }
        }
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        if !self.execute_scripts {
            return Ok(());
        }
        if let Some(mut backend) = engine.remove_resource::<Box<dyn ScriptingBackend>>() {
            backend.on_update(engine, dt);
            engine.insert_resource(backend);
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoring_only_mode_keeps_script_components_inert() {
        let mut engine = Engine::new();
        let actor = engine
            .spawn_actor("Authored Script")
            .with(ScriptComponent {
                script: "assets/scripts/does_not_exist.lua".to_owned(),
                enabled: true,
                ..ScriptComponent::default()
            })
            .build();

        let mut plugin = LuaScriptingPlugin::authoring_only();
        plugin.initialize(&mut engine).unwrap();
        plugin.update(&mut engine, 1.0 / 60.0).unwrap();

        assert!(engine.raw_world().get::<ScriptComponent>(actor.entity()).is_some());
        let state = engine.get_resource::<LuaScriptingState>().unwrap();
        assert!(state.scripts.is_empty());
        assert!(state.entity_scripts.is_empty());
        assert!(state.instances.is_empty());
    }
}
