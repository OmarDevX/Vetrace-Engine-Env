use vetrace_engine::Behaviour;
use vetrace_engine::components::components::{
    Atmosphere, Bloom, CameraAttachment, DirectionalLight, FreeFlightControls,
    PostProcessing, Transform,
};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;

fn main() {
    // Start the engine with 3D rendering
    let mut engine = Engine::new(false);

    // Spawn a planet so the atmosphere has something to wrap around
    let mut planet = Object::default();
    planet.is_cube = false; // sphere
    planet.radius = 100.0;
    planet.position = [0.0, -planet.radius, 0.0];
    planet.color = [0.0, 64.0, 12.0];
    if let Some(mut actor) = engine.spawn_object_as_actor(planet) {
        actor.with_bundle(Atmosphere::default());
    }

    // A second planet with its own atmosphere to demonstrate multiple instances
    let mut planet2 = Object::default();
    planet2.is_cube = false;
    planet2.radius = 50.0;
    planet2.position = [200.0, -planet2.radius, 0.0];
    planet2.color = [64.0, 16.0, 64.0];
    if let Some(mut actor) = engine.spawn_object_as_actor(planet2) {
        let mut atmo = Atmosphere::default();
        atmo.planet_radius = 50.0;
        atmo.atmo_radius = 60.0;
        actor.with_bundle(atmo);
    }

    // Spawn a cube with a high emission value above the surface
    let mut cube = Object::default();
    cube.position = [0.0, 1.0, 0.0];
    cube.color = [255.0, 140.0, 60.0];
    cube.emission = 0.0; // extremely bright
    engine.spawn_object(cube);

    // Create a camera looking at the cube
    let cam = engine.spawn_empty("camera");
    engine.world.insert(
        cam,
        Transform {
            position: [0.0, 0.0, -5.0],
            ..Default::default()
        },
    );

    // Enable bloom so the emission appears as a glow
    let bloom = Bloom {
        threshold: 1.0,
        intensity: 2.0,
        spread: 4.0,
        iterations: 7,
        ..Default::default()
    };
    engine.world.insert(
        cam,
        PostProcessing {
            bloom: Some(bloom),
            ..Default::default()
        },
    );

    // Add a basic directional light and controls
    engine.world.insert(
        cam,
        DirectionalLight {
            direction: [-1.0, -1.0, -1.0],
            color: [255.0, 255.0, 255.0],
            intensity: 1.0,
        },
    );
    engine.world.insert(cam, CameraAttachment::default());
    engine.world.insert(cam, FreeFlightControls::default());


    engine.run(true);
}

struct DayNightCycle {
    ent: Entity,
    time: f32,
}

impl Behaviour for DayNightCycle {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        self.time += delta * 0.25;
        let diff = 100.0; // atmosphere thickness in world units
        if let Some(transform) = engine.world.get_mut::<Transform>(self.ent) {
            transform.position[1] = diff + (-self.time.cos()) * (diff - 1.0);
        }
        if let Some(light) = engine.world.get_mut::<DirectionalLight>(self.ent) {
            let dir = [0.0, -self.time.cos(), -self.time.sin()];
            let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
            light.direction = [dir[0] / len, dir[1] / len, dir[2] / len];
        }
    }
}