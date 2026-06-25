use vetrace_engine::app::{app, App};
use vetrace_engine::behaviour::look_at::LookAtBehaviour;
use vetrace_engine::components::components::{
    Collider, LookAt, Metadata, Renderable, ScriptComponent, UILabel, UILayout, UIScreenSpace, Velocity
};
use vetrace_engine::ecs::Component;
use vetrace_engine::engine::Engine;
use vetrace_engine::inspector::export::{ExportKind, ExportedField};
use vetrace_engine::inspector::Inspectable;
use vetrace_engine::scene::object::Object;
use vetrace_engine_macros::Inspectable;

#[derive(Debug, Default, Inspectable)]
pub struct Score {
    #[export]
    pub value: i32,
}

impl Component for Score {}

struct TopDownLua;

impl App for TopDownLua {
    fn setup(&mut self, engine: &mut Engine) {
        engine.reload_scripts();
        engine.add_behaviour(LookAtBehaviour);
        // Register UI components so prefabs can instantiate them. The names
        // must match those used in the JSON files.
        engine.auto_register_component::<UILabel>("UILabel");
        engine.auto_register_component::<UILayout>("UILayout");
        engine.auto_register_component::<UIScreenSpace>("UIScreenSpace");
        engine.auto_register_component::<Score>("Score");
        engine.sky_color= [0.0, 0.0, 0.0];
        let mut player_obj = Object::default();
        player_obj.is_static = false;
        player_obj.size = [0.5, 0.5, 0.5];
        player_obj.color = [0.0, 150.0, 255.0];
        if let Some(mut actor) = engine.spawn_object_as_actor(player_obj) {
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = "Player".into();
                meta.tags.push("player".into());
            }
            actor.with_bundle((
                Velocity::default(),
                Collider::default(),
                LookAt::default(),
                ScriptComponent {
                    script: "player".into(),
                },
            ));
            actor.add_component::<Score>();
            if let Some(look) = actor.get_component_mut::<LookAt>() {
                look.target = "mouse".into();
                look.rotate_x = true;
                look.rotate_y = true;
                look.rotate_z = true;
            }
        }
        
        let mut game_obj = Object::default();
        game_obj.is_static = true;
        game_obj.size = [0.1, 0.1, 0.1];
        if let Some(mut actor) = engine.spawn_object_as_actor(game_obj) {
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = "GameController".into();
            }
            actor.add_component::<ScriptComponent>();
            if let Some(script) = actor.get_component_mut::<ScriptComponent>() {
                script.script = "game".into();
            }
        }
    }

    fn update(&mut self, _engine: &mut Engine, _delta: f32) {}

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app().with_title("Top Down Lua").run(TopDownLua)
}
