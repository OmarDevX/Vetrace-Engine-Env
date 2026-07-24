use std::any::Any;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use uuid::Uuid;
use vetrace_core::{Engine, Plugin};
use vetrace_project::{InputAction, ProjectManifest, ProjectPath, VetraceProject};
use vetrace_runtime::{
    ActiveRuntimeScene, RuntimeAutoloads, RuntimeError, RuntimeInputMap, RuntimeMode,
    RuntimeState, VetraceRuntime,
};
use vetrace_scene::{save_scene_file, SceneComponent, SceneDocument, SceneNode, SceneTransform};
use vetrace_scripting_lua::{LuaScriptingState, ScriptComponent};

struct TestProject {
    root: PathBuf,
    project: VetraceProject,
}

impl TestProject {
    fn create(with_scene_script: bool, with_autoload: bool) -> Self {
        let root = std::env::temp_dir().join(format!("vetrace-runtime-test-{}", Uuid::new_v4()));
        let mut manifest = ProjectManifest::new("Runtime Test", "0.1.0");
        manifest.features.networking = false;
        manifest.features.audio = false;
        manifest.features.animation = false;
        manifest.input.insert("jump", InputAction {
            keys: vec!["Space".to_owned()],
            ..InputAction::default()
        });
        if with_autoload {
            manifest.runtime.autoload_scripts = vec![ProjectPath::new("assets/scripts/game.lua").unwrap()];
        }

        let project = VetraceProject::create(&root, manifest).unwrap();
        if with_autoload {
            fs::write(
                root.join("assets/scripts/game.lua"),
                "return { start = function(engine) end, update = function(engine, input, dt) end }",
            )
            .unwrap();
        }
        if with_scene_script {
            fs::write(
                root.join("assets/scripts/player.lua"),
                "return { start = function(engine, entity) end, update = function(engine, entity, input, dt) end }",
            )
            .unwrap();
        }

        let components = if with_scene_script {
            vec![SceneComponent::new(
                "vetrace.scripting.lua_script",
                ScriptComponent {
                    script: "assets/scripts/player.lua".to_owned(),
                    ..ScriptComponent::default()
                },
            )]
        } else {
            Vec::new()
        };
        let mut scene = SceneDocument::new("Main");
        scene.roots.push(SceneNode {
            id: Uuid::new_v4().to_string(),
            name: "Root".to_owned(),
            transform: SceneTransform::default(),
            components,
            children: Vec::new(),
        });
        save_scene_file(root.join("assets/scenes/main.vscene"), &scene).unwrap();

        Self { root, project }
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn headless_runtime_loads_project_and_scene() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();

    assert_eq!(runtime.state(), RuntimeState::Created);
    runtime.start().unwrap();
    assert_eq!(runtime.state(), RuntimeState::Running);

    let active = runtime.engine().get_resource::<ActiveRuntimeScene>().unwrap();
    assert_eq!(active.document.name, "Main");
    assert_eq!(active.instance.actors.len(), 1);
    assert_eq!(runtime.engine().actors().len(), 1);
}

#[test]
fn runtime_loads_autoload_and_scene_lua_scripts() {
    let fixture = TestProject::create(true, true);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();
    runtime.start().unwrap();

    let autoloads = runtime.engine().get_resource::<RuntimeAutoloads>().unwrap();
    assert_eq!(autoloads.scripts, vec![ProjectPath::new("assets/scripts/game.lua").unwrap()]);

    let active = runtime.engine().get_resource::<ActiveRuntimeScene>().unwrap();
    let entity = active.instance.actors[0].entity();
    let component = runtime.engine().raw_world().get::<ScriptComponent>(entity).unwrap();
    assert_eq!(component.script, "assets/scripts/player.lua");

    let state = runtime.engine().get_resource::<LuaScriptingState>().unwrap();
    assert!(state.scripts.contains_key("assets/scripts/game.lua"));
    assert!(state.scripts.contains_key("assets/scripts/player.lua"));
    assert_eq!(state.entity_scripts.get(&entity).map(String::as_str), Some("assets/scripts/player.lua"));

    runtime.update(1.0 / 60.0).unwrap();
    assert!(runtime.engine().get_resource::<LuaScriptingState>().unwrap().started_scripts.contains(&entity));
}


#[test]
fn editor_preview_can_load_scene_without_running_project_scripts() {
    let fixture = TestProject::create(true, true);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .run_project_scripts(false)
        .build()
        .unwrap();
    runtime.start().unwrap();

    assert!(runtime.engine().get_resource::<RuntimeAutoloads>().is_none());
    let active = runtime.engine().get_resource::<ActiveRuntimeScene>().unwrap();
    let entity = active.instance.actors[0].entity();
    let _component = runtime.engine().raw_world().get::<ScriptComponent>(entity).unwrap();
    let state = runtime.engine().get_resource::<LuaScriptingState>().unwrap();
    assert!(!state.started_scripts.contains(&entity));
    assert!(state.scripts.is_empty());
    assert!(state.entity_scripts.is_empty());
}

#[test]
fn reload_replaces_instead_of_duplicating_scene() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();
    runtime.start().unwrap();
    let first_entity = runtime
        .engine()
        .get_resource::<ActiveRuntimeScene>()
        .unwrap()
        .instance
        .actors[0]
        .entity();

    runtime.reload_scene().unwrap();

    let active = runtime.engine().get_resource::<ActiveRuntimeScene>().unwrap();
    let second_entity = active.instance.actors[0].entity();
    assert_ne!(first_entity, second_entity);
    assert!(!runtime.engine().raw_world().is_alive(first_entity));
    assert_eq!(runtime.engine().actors().len(), 1);
}

#[test]
fn pause_resume_and_stop_are_explicit() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();
    runtime.start().unwrap();
    runtime.pause().unwrap();
    assert_eq!(runtime.state(), RuntimeState::Paused);
    runtime.update(1.0).unwrap();
    runtime.resume().unwrap();
    assert_eq!(runtime.state(), RuntimeState::Running);
    runtime.stop().unwrap();
    assert_eq!(runtime.state(), RuntimeState::Stopped);
    assert!(runtime.engine().get_resource::<ActiveRuntimeScene>().is_none());
}

#[test]
fn project_input_actions_are_available_as_runtime_resource() {
    let fixture = TestProject::create(false, false);
    let runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();
    let actions = runtime.engine().get_resource::<RuntimeInputMap>().unwrap();
    assert_eq!(actions.action("jump").unwrap().keys, vec!["Space".to_owned()]);
}

#[derive(Default)]
struct MarkerPlugin;
#[derive(Debug, PartialEq, Eq)]
struct MarkerResource(&'static str);

impl Plugin for MarkerPlugin {
    fn name(&self) -> &'static str { "runtime_test_marker" }
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource(MarkerResource("installed"));
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[test]
fn runtime_accepts_generic_extra_plugins() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .add_plugin(MarkerPlugin)
        .build()
        .unwrap();
    runtime.start().unwrap();
    assert_eq!(
        runtime.engine().get_resource::<MarkerResource>().unwrap().0,
        "installed"
    );
}

#[test]
fn missing_scene_fails_before_runtime_creation() {
    let fixture = TestProject::create(false, false);
    fs::remove_file(fixture.root.join("assets/scenes/main.vscene")).unwrap();
    let result = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build();
    assert!(result.is_err());
}


#[test]
fn completed_frames_update_runtime_status() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();

    runtime.update(0.25).unwrap();

    assert_eq!(runtime.frame_count(), 1);
    let status = runtime.status().unwrap();
    assert_eq!(status.frame, 1);
    assert_eq!(status.delta_seconds, 0.25);
    assert_eq!(status.elapsed.as_secs_f32(), 0.25);
}

#[test]
fn invalid_delta_is_rejected_without_advancing() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .build()
        .unwrap();

    assert!(matches!(runtime.update(f32::NAN), Err(RuntimeError::InvalidDelta(_))));
    assert_eq!(runtime.state(), RuntimeState::Created);
    assert_eq!(runtime.frame_count(), 0);
}

#[derive(Default)]
struct ExternalScriptEntityPlugin;

impl Plugin for ExternalScriptEntityPlugin {
    fn name(&self) -> &'static str { "external_script_entity" }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        let actor = engine.spawn_actor("External Runtime Entity").build();
        actor.insert(engine, ScriptComponent {
            script: "not/a/project/scene/script.txt".to_owned(),
            ..ScriptComponent::default()
        })?;
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[test]
fn scene_script_binding_ignores_entities_outside_the_scene_instance() {
    let fixture = TestProject::create(false, false);
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .add_plugin(ExternalScriptEntityPlugin)
        .build()
        .unwrap();

    runtime.start().unwrap();
    assert_eq!(runtime.active_scene().unwrap().instance.actors.len(), 1);
    assert_eq!(runtime.engine().actors().len(), 2);
}

#[test]
fn startup_preserves_typed_scene_errors() {
    let fixture = TestProject::create(false, false);
    fs::write(fixture.root.join("assets/scenes/main.vscene"), "not valid scene json").unwrap();
    let mut runtime = VetraceRuntime::builder(fixture.project.clone())
        .mode(RuntimeMode::Test)
        .validate_project_files(false)
        .build()
        .unwrap();

    let error = runtime.start().unwrap_err();
    assert!(matches!(error, RuntimeError::SceneLoad { .. }));
    assert_eq!(runtime.state(), RuntimeState::Failed);
}
