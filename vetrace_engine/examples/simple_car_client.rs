use vetrace_engine::components::components::ScriptComponent;
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;

fn main() {
    let mut engine = Engine::new(false);
    engine.reload_scripts();
    let mut obj = Object::default();
    obj.is_static = true;
    if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
        actor.add_component::<ScriptComponent>();
        if let Some(script) = actor.get_component_mut::<ScriptComponent>() {
            script.script = "simple_car_client".into();
        }
    }
    engine.run(true);
}
