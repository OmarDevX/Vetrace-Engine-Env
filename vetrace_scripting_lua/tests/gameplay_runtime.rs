use std::collections::BTreeMap;

use mlua::Table;
use vetrace_core::{Engine, Transform};
use vetrace_scripting_lua::{
    attach_autoload_script, attach_loaded_script, detach_script, fixed_update_scripts,
    load_script_from_file_as, start_pending_autoload_scripts, start_pending_scripts,
    update_autoload_scripts, update_scripts, LuaDiagnostics, LuaScriptInstanceStatus,
    LuaProjectContext, LuaScriptingState, ScriptComponent, ScriptValue,
};
use vetrace_project::VetraceProject;

fn load_source(engine: &mut Engine, name: &str, source: &str) {
    let path = std::env::temp_dir().join(format!(
        "vetrace-lua-test-{}-{}.lua",
        std::process::id(),
        name.replace('/', "_")
    ));
    std::fs::write(&path, source).unwrap();
    load_script_from_file_as(engine, &path, name).unwrap();
    let _ = std::fs::remove_file(path);
}

fn scripted_actor(
    engine: &mut Engine,
    name: &str,
    script: &str,
    properties: BTreeMap<String, ScriptValue>,
) -> vetrace_core::Entity {
    let actor = engine
        .spawn_actor(name)
        .with(Transform::default())
        .with(ScriptComponent {
            script: script.to_owned(),
            enabled: true,
            properties,
        })
        .build();
    attach_loaded_script(engine, actor.entity(), script.to_owned());
    actor.entity()
}


#[test]
fn project_modules_are_cached_inside_each_script_environment() {
    let root = std::env::temp_dir().join(format!(
        "vetrace-lua-modules-{}",
        std::process::id(),
    ));
    let _ = std::fs::remove_dir_all(&root);
    let project = VetraceProject::create_new(&root, "Lua Modules Test", "0.1.0").unwrap();
    let helper = root.join("assets/scripts/helper.lua");
    let main = root.join("assets/scripts/main.lua");
    std::fs::write(
        &helper,
        r#"
        return {
            value = 41,
            increment = function(value) return value + 1 end,
        }
        "#,
    )
    .unwrap();
    std::fs::write(
        &main,
        r#"
        return {
            ready = function(self)
                local first = Modules.require("assets/scripts/helper.lua")
                local second = Modules.require("assets/scripts/helper.lua")
                self.same_module = first == second
                self.result = first.increment(first.value)
            end,
        }
        "#,
    )
    .unwrap();

    let mut engine = Engine::new();
    engine.insert_resource(LuaProjectContext::new(project));
    load_script_from_file_as(&mut engine, &main, "main").unwrap();
    attach_autoload_script(&mut engine, "main");
    start_pending_autoload_scripts(&mut engine);

    let state = engine.get_resource::<LuaScriptingState>().unwrap();
    let table: Table = state
        .lua
        .registry_value(&state.autoload_instances["main"].table)
        .unwrap();
    assert!(table.get::<bool>("same_module").unwrap());
    assert_eq!(table.get::<i64>("result").unwrap(), 42);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn entity_instances_keep_isolated_state_and_property_overrides() {
    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "mover",
        r#"
        return {
            properties = {
                step = { type = "number", default = 1.0 },
            },
            ready = function(self)
                self.counter = 0.0
                self.fixed_ticks = 0
            end,
            update = function(self, _dt)
                self.counter = self.counter + self.step
                self.transform.x = self.counter
            end,
            fixed_update = function(self, _dt)
                self.fixed_ticks = self.fixed_ticks + 1
                self.transform.y = self.fixed_ticks
            end,
        }
        "#,
    );

    let first = scripted_actor(&mut engine, "First", "mover", BTreeMap::new());
    let second = scripted_actor(
        &mut engine,
        "Second",
        "mover",
        BTreeMap::from([("step".to_owned(), ScriptValue::Number(3.0))]),
    );

    start_pending_scripts(&mut engine);
    update_scripts(&mut engine, 1.0 / 60.0);
    fixed_update_scripts(&mut engine, 1.0 / 60.0);

    assert_eq!(engine.actor(first).unwrap().transform(&engine).unwrap().translation.x, 1.0);
    assert_eq!(engine.actor(second).unwrap().transform(&engine).unwrap().translation.x, 3.0);
    assert_eq!(engine.actor(first).unwrap().transform(&engine).unwrap().translation.y, 1.0);
    assert_eq!(engine.actor(second).unwrap().transform(&engine).unwrap().translation.y, 1.0);

    let state = engine.get_resource::<LuaScriptingState>().unwrap();
    let first_table: Table = state.lua.registry_value(&state.instances[&first].table).unwrap();
    let second_table: Table = state.lua.registry_value(&state.instances[&second].table).unwrap();
    assert_eq!(first_table.get::<f64>("counter").unwrap(), 1.0);
    assert_eq!(second_table.get::<f64>("counter").unwrap(), 3.0);
    assert_eq!(state.instances[&first].status, LuaScriptInstanceStatus::Running);
    assert_eq!(state.instances[&second].status, LuaScriptInstanceStatus::Running);
}

#[test]
fn structural_commands_are_deferred_and_destroy_runs_before_detach() {
    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "autoload",
        r#"
        return {
            ready = function(self)
                self.done = false
            end,
            update = function(self, _dt)
                if not self.done then
                    local spawned = Scene.spawn("Deferred Spawn")
                    spawned:add_tag("lua_spawned")
                    spawned:set_translation(4.0, 5.0, 6.0)
                    self.done = true
                end
            end,
        }
        "#,
    );
    attach_autoload_script(&mut engine, "autoload");
    start_pending_autoload_scripts(&mut engine);
    assert_eq!(engine.actors().len(), 0);
    update_autoload_scripts(&mut engine, 1.0 / 60.0);

    let spawned = engine.find_actor_by_name("Deferred Spawn").unwrap();
    assert!(spawned.has_tag(&engine, "lua_spawned"));
    assert_eq!(spawned.transform(&engine).unwrap().translation, glam::Vec3::new(4.0, 5.0, 6.0));

    load_source(
        &mut engine,
        "destroyable",
        r#"
        return {
            ready = function(self) end,
            destroy = function(self)
                Scene.spawn("Destroy Callback Marker")
            end,
        }
        "#,
    );
    let entity = scripted_actor(&mut engine, "Destroyable", "destroyable", BTreeMap::new());
    start_pending_scripts(&mut engine);
    detach_script(&mut engine, entity);
    assert!(engine.find_actor_by_name("Destroy Callback Marker").is_some());
}

#[test]
fn a_failing_instance_does_not_stop_other_instances() {
    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "isolated_error",
        r#"
        return {
            properties = {
                fail = { type = "boolean", default = false },
            },
            ready = function(self) end,
            update = function(self, _dt)
                if self.fail then
                    error("intentional instance failure")
                end
                self.transform.x = self.transform.x + 1.0
            end,
        }
        "#,
    );

    let failing = scripted_actor(
        &mut engine,
        "Failing",
        "isolated_error",
        BTreeMap::from([("fail".to_owned(), ScriptValue::Bool(true))]),
    );
    let healthy = scripted_actor(&mut engine, "Healthy", "isolated_error", BTreeMap::new());
    start_pending_scripts(&mut engine);
    update_scripts(&mut engine, 1.0 / 60.0);
    update_scripts(&mut engine, 1.0 / 60.0);

    assert_eq!(engine.actor(healthy).unwrap().transform(&engine).unwrap().translation.x, 2.0);
    assert_eq!(engine.actor(failing).unwrap().transform(&engine).unwrap().translation.x, 0.0);
    let state = engine.get_resource::<LuaScriptingState>().unwrap();
    assert_eq!(state.instances[&failing].status, LuaScriptInstanceStatus::Failed);
    assert_eq!(state.instances[&healthy].status, LuaScriptInstanceStatus::Running);
    assert_eq!(engine.get_resource::<LuaDiagnostics>().unwrap().errors().len(), 1);
}

#[test]
fn event_and_collision_hooks_are_dispatchable() {
    use vetrace_scripting_lua::{
        dispatch_collision_enter, dispatch_collision_exit, dispatch_script_event,
    };

    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "events",
        r#"
        return {
            ready = function(self)
                self.events = 0
                self.enters = 0
                self.exits = 0
            end,
            on_event = function(self, name, payload)
                if name == "score" then
                    self.events = self.events + payload
                end
            end,
            on_collision_enter = function(self, _other)
                self.enters = self.enters + 1
            end,
            on_collision_exit = function(self, _other)
                self.exits = self.exits + 1
            end,
        }
        "#,
    );
    let entity = scripted_actor(&mut engine, "Listener", "events", BTreeMap::new());
    let other = engine.spawn_actor("Other").build().entity();
    start_pending_scripts(&mut engine);

    dispatch_script_event(&mut engine, entity, "score", ScriptValue::Integer(5));
    dispatch_collision_enter(&mut engine, entity, other);
    dispatch_collision_exit(&mut engine, entity, other);

    let state = engine.get_resource::<LuaScriptingState>().unwrap();
    let table: Table = state.lua.registry_value(&state.instances[&entity].table).unwrap();
    assert_eq!(table.get::<i64>("events").unwrap(), 5);
    assert_eq!(table.get::<i64>("enters").unwrap(), 1);
    assert_eq!(table.get::<i64>("exits").unwrap(), 1);
}

#[test]
fn hot_reload_restarts_only_instances_using_the_reloaded_template() {
    use vetrace_scripting_lua::reload_script_from_file_as;

    let mut engine = Engine::new();
    let path = std::env::temp_dir().join(format!(
        "vetrace-lua-hot-reload-{}.lua",
        std::process::id()
    ));
    std::fs::write(
        &path,
        r#"
        return {
            ready = function(self) self.transform.x = 1.0 end,
            update = function(self, _dt) end,
        }
        "#,
    )
    .unwrap();
    load_script_from_file_as(&mut engine, &path, "reloadable").unwrap();
    let entity = scripted_actor(&mut engine, "Reloadable", "reloadable", BTreeMap::new());
    start_pending_scripts(&mut engine);
    assert_eq!(engine.actor(entity).unwrap().transform(&engine).unwrap().translation.x, 1.0);

    std::fs::write(&path, "return { update = function(").unwrap();
    assert!(reload_script_from_file_as(&mut engine, &path, "reloadable").is_err());
    let state = engine.get_resource::<LuaScriptingState>().unwrap();
    assert_eq!(state.instances[&entity].status, LuaScriptInstanceStatus::Running);
    assert!(engine.get_resource::<LuaScriptingState>().unwrap().started_scripts.contains(&entity));

    std::fs::write(
        &path,
        r#"
        return {
            ready = function(self) self.transform.x = 7.0 end,
            update = function(self, _dt) end,
        }
        "#,
    )
    .unwrap();
    reload_script_from_file_as(&mut engine, &path, "reloadable").unwrap();
    start_pending_scripts(&mut engine);
    assert_eq!(engine.actor(entity).unwrap().transform(&engine).unwrap().translation.x, 7.0);
    let _ = std::fs::remove_file(path);
}

#[test]
fn input_actions_are_available_without_runtime_crate_dependency() {
    use vetrace_scripting_lua::{LuaInputAction, LuaInputMap};

    let mut engine = Engine::new();
    engine.insert_resource(LuaInputMap::new(BTreeMap::from([(
        "move_right".to_owned(),
        LuaInputAction { keys: vec!["D".to_owned()], mouse_buttons: Vec::new() },
    )])));
    engine.get_resource_mut::<vetrace_core::InputState>().unwrap().set_key_down("D", true);
    load_source(
        &mut engine,
        "input_action",
        r#"
        return {
            ready = function(self) end,
            update = function(self, dt)
                if Input.action_down("move_right") then
                    self.transform:translate_xyz(5.0 * dt, 0.0, 0.0)
                end
            end,
        }
        "#,
    );
    let entity = scripted_actor(&mut engine, "Input", "input_action", BTreeMap::new());
    start_pending_scripts(&mut engine);
    update_scripts(&mut engine, 0.2);
    assert_eq!(engine.actor(entity).unwrap().transform(&engine).unwrap().translation.x, 1.0);
}

#[test]
fn update_only_scripts_default_to_the_gameplay_api() {
    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "update_only",
        r#"
        return {
            step = 2.0,
            apply_step = function(self)
                self.transform.x = self.transform.x + self.step
            end,
            update = function(self, _dt)
                self:apply_step()
            end,
        }
        "#,
    );
    let entity = scripted_actor(&mut engine, "Update Only", "update_only", BTreeMap::new());
    start_pending_scripts(&mut engine);
    update_scripts(&mut engine, 1.0 / 60.0);
    assert_eq!(engine.actor(entity).unwrap().transform(&engine).unwrap().translation.x, 2.0);
}

#[test]
fn disabling_and_reenabling_a_component_restarts_its_instance_cleanly() {
    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "toggleable",
        r#"
        return {
            ready = function(self)
                self.generation = (self.generation or 0) + 1
                self.transform.x = self.transform.x + 1.0
            end,
            update = function(self, _dt) end,
            destroy = function(self)
                self.transform.y = self.transform.y + 1.0
            end,
        }
        "#,
    );
    let entity = scripted_actor(&mut engine, "Toggleable", "toggleable", BTreeMap::new());
    start_pending_scripts(&mut engine);

    engine.raw_world_mut().get_mut::<ScriptComponent>(entity).unwrap().enabled = false;
    update_scripts(&mut engine, 1.0 / 60.0);
    assert_eq!(engine.actor(entity).unwrap().transform(&engine).unwrap().translation.y, 1.0);
    assert!(!engine.get_resource::<LuaScriptingState>().unwrap().started_scripts.contains(&entity));

    engine.raw_world_mut().get_mut::<ScriptComponent>(entity).unwrap().enabled = true;
    start_pending_scripts(&mut engine);
    assert_eq!(engine.actor(entity).unwrap().transform(&engine).unwrap().translation.x, 2.0);
    assert!(engine.get_resource::<LuaScriptingState>().unwrap().started_scripts.contains(&entity));
}

#[test]
fn autoload_destroy_runs_during_explicit_shutdown() {
    use vetrace_scripting_lua::shutdown_autoload_scripts;

    let mut engine = Engine::new();
    load_source(
        &mut engine,
        "shutdown_autoload",
        r#"
        return {
            ready = function(self) end,
            destroy = function(self)
                Scene.spawn("Autoload Destroy Marker")
            end,
        }
        "#,
    );
    attach_autoload_script(&mut engine, "shutdown_autoload");
    start_pending_autoload_scripts(&mut engine);
    shutdown_autoload_scripts(&mut engine);
    assert!(engine.find_actor_by_name("Autoload Destroy Marker").is_some());
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct RegenSettings {
    rate: f32,
}

#[derive(
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
    vetrace_core::VetraceComponent,
)]
#[vetrace_component(id = "test.health", display_name = "Health", category = "Gameplay")]
struct ReflectedHealth {
    #[vetrace(min = 0.0, max = 100.0)]
    current: f32,
    maximum: f32,
    regen: RegenSettings,
}

#[derive(
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
    vetrace_core::VetraceComponent,
)]
#[vetrace_component(id = "test.mana", display_name = "Mana", category = "Gameplay")]
struct ReflectedMana {
    current: f32,
    maximum: f32,
}

#[test]
fn lua_reflection_accesses_custom_components_without_lua_type_bindings() {
    let mut engine = Engine::new();
    {
        let registry = engine.get_resource_mut::<vetrace_core::ComponentManager>().unwrap();
        registry.register_reflected::<ReflectedHealth>();
        registry.register_reflected::<ReflectedMana>();
    }

    load_source(
        &mut engine,
        "generic_components",
        r#"
        return {
            ready = function(self)
                assert(self:has_component("test.health"))
                assert(self.components.Health ~= nil)
                self.components.Health.current = 40.0
                self.components.Health.regen.rate = 2.5
                self.components.Transform.translation.x = 6.0
                self:add_component("test.mana", {
                    current = 12.0,
                    maximum = 50.0,
                })
            end,
            update = function(self, _dt)
                local mana = self:get_component("Mana")
                if mana ~= nil then
                    mana.current = mana.current + 3.0
                    self.observed_mana = mana.current
                    self:remove_component("test.mana")
                end
            end,
        }
        "#,
    );

    let actor = engine
        .spawn_actor("Reflected Lua Actor")
        .with(ReflectedHealth {
            current: 100.0,
            maximum: 100.0,
            regen: RegenSettings { rate: 1.0 },
        })
        .with(ScriptComponent {
            script: "generic_components".to_owned(),
            enabled: true,
            properties: BTreeMap::new(),
        })
        .build();
    attach_loaded_script(&mut engine, actor.entity(), "generic_components");

    start_pending_scripts(&mut engine);
    assert_eq!(actor.get_component::<ReflectedHealth>(&engine).unwrap().current, 40.0);
    assert_eq!(actor.get_component::<ReflectedHealth>(&engine).unwrap().regen.rate, 2.5);
    assert_eq!(actor.transform(&engine).unwrap().translation.x, 6.0);
    assert_eq!(actor.get_component::<ReflectedMana>(&engine).unwrap().current, 12.0);

    update_scripts(&mut engine, 1.0 / 60.0);
    assert!(!actor.has::<ReflectedMana>(&engine));
    let state = engine.get_resource::<LuaScriptingState>().unwrap();
    let table: Table = state.lua.registry_value(&state.instances[&actor.entity()].table).unwrap();
    assert_eq!(table.get::<f64>("observed_mana").unwrap(), 15.0);
}
