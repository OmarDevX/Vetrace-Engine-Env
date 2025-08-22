use vetrace_engine::app::{run_app, App, AppConfig};
use vetrace_engine::components::components::{
    Anchor, Metadata, ScriptComponent, UIButton, UILabel, UILayout, UIPanel, UIScreenSpace,
    UITextEditor,
};
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;

struct UiExample;

impl App for UiExample {
    fn on_start(&mut self, engine: &mut Engine) {
        engine.auto_register_component::<UILabel>("UILabel");
        engine.auto_register_component::<UILayout>("UILayout");
        engine.auto_register_component::<UIScreenSpace>("UIScreenSpace");
        engine.auto_register_component::<UIPanel>("UIPanel");
        engine.auto_register_component::<UIButton>("UIButton");
        engine.auto_register_component::<UITextEditor>("UITextEditor");
        engine.auto_register_component::<Metadata>("Metadata");
        engine.auto_register_component::<ScriptComponent>("Script");

        // Label entity
        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UILabel>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = "Label".into();
            }
        }

        // Text editor entity
        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UITextEditor>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::TopLeft;
                layout.offset = [10.0, 10.0];
            }
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = "Editor".into();
            }
        }

        // Submit button entity
        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UIButton>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            actor.add_component::<ScriptComponent>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::TopLeft;
                layout.offset = [10.0, 50.0];
            }
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = "Button".into();
            }
            if let Some(btn) = actor.get_component_mut::<UIButton>() {
                btn.text = "Show Text".into();
                btn.size = [120.0, 24.0];
            }
            if let Some(script) = actor.get_component_mut::<ScriptComponent>() {
                script.script = "ui_example".into();
            }
        }

        // Clear button entity
        let mut obj = Object::default();
        obj.is_static = true;
        if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
            actor.add_component::<UIButton>();
            actor.add_component::<UILayout>();
            actor.add_component::<UIScreenSpace>();
            actor.add_component::<ScriptComponent>();
            if let Some(layout) = actor.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::TopLeft;
                layout.offset = [140.0, 50.0];
            }
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = "ClearButton".into();
            }
            if let Some(btn) = actor.get_component_mut::<UIButton>() {
                btn.text = "Clear".into();
                btn.size = [100.0, 24.0];
            }
            if let Some(script) = actor.get_component_mut::<ScriptComponent>() {
                script.script = "ui_example".into();
            }
        }

        // Load scripts so Lua behaviours are ready
        engine.reload_scripts();
    }

    fn on_update(&mut self, _engine: &mut Engine, _delta: f32) {}
}

fn main() {
    // Game UI works without the built-in editor interface
    run_app(
        UiExample,
        AppConfig {
            is_2d: true,
            enable_editor: false,
        },
    );
}
