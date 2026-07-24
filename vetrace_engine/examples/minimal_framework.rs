use vetrace_engine::prelude::*;

struct Game;

impl App for Game {
    fn setup(&mut self, engine: &mut Engine) {
        let player = engine
            .spawn_actor("Player")
            .with(Transform::default())
            .build();
        println!("spawned {:?}", player);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new().run(Game)
}
