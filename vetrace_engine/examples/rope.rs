use vetrace_engine::components::components::{
    BallJoint, Collider, ColliderShape, RigidBody3D, StaticBody,
};
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;

fn main() {
    let mut engine = Engine::new(false);

    // Ground plane
    let mut ground = Object::default();
    ground.position = [0.0, -2.0, 0.0];
    ground.size = [20.0, 1.0, 20.0];
    ground.is_cube = true;
    engine.spawn_object(ground);
    let ground_id = (engine.scene.objects.len() - 1) as u32;
    if let Some(ent) = engine.core.find_entity_by_object_id(ground_id) {
        engine.world.insert(ent, StaticBody::default());
    }

    // Static hook at the top of the rope
    let mut hook = Object::default();
    hook.position = [0.0, 4.0, 0.0];
    hook.size = [0.2, 0.2, 0.2];
    hook.is_cube = true;
    engine.spawn_object(hook);
    let hook_id = (engine.scene.objects.len() - 1) as u32;
    if let Some(ent) = engine.core.find_entity_by_object_id(hook_id) {
        engine.world.insert(ent, StaticBody::default());
    }

    // Chain of cubes hanging from the hook
    let mut prev = hook_id;
    let links = 5;
    for i in 0..links {
        let mut link = Object::default();
        link.position = [0.0, 3.8 - i as f32 * 0.6, 0.0];
        link.size = [0.4, 0.4, 0.4];
        link.is_cube = true;
        engine.spawn_object(link);
        let link_id = (engine.scene.objects.len() - 1) as u32;
        if let Some(ent) = engine.core.find_entity_by_object_id(link_id) {
            engine.world.insert(ent, RigidBody3D::default());
            engine.world.insert(
                ent,
                BallJoint {
                    other: prev,
                    contacts_enabled: false,
                    handle: None,
                },
            );
            if let Some(col) = engine.world.get_mut::<Collider>(ent) {
                col.shape = ColliderShape::Cube;
            }
        }
        prev = link_id;
    }

    engine.run(true);
}
