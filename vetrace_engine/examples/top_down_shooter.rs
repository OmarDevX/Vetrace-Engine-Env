use rand::Rng;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use vetrace_engine::app::{app, App};
use vetrace_engine::behaviour::look_at::LookAtBehaviour;
use vetrace_engine::components::components::{Anchor, UILabel, UILayout, UIScreenSpace};
use vetrace_engine::components::components::{
    Collider, Lifetime, LookAt, Metadata, Player, Transform, Velocity,
};
use vetrace_engine::ecs::{Component, Entity};
use vetrace_engine::inspector::export::{ExportKind, ExportedField};
use vetrace_engine::inspector::Inspectable;
use vetrace_engine::scene::object::Object;
use vetrace_engine::Behaviour;
use vetrace_engine::{Engine, Prefab};
use vetrace_engine_macros::Inspectable;
#[derive(Debug, Inspectable)]
pub struct CustomPlayer {
    #[export]
    pub acceleration: Option<f32>,
    #[export]
    pub damping: Option<f32>,
    #[export]
    pub velocity: Option<[f32; 2]>,
    #[export]
    pub speed: f32,
}

impl Component for CustomPlayer {}
impl Default for CustomPlayer {
    fn default() -> Self {
        Self {
            speed: 45.0,
            acceleration: Some(1.0),
            damping: Some(0.9),
            velocity: Some([0.0, 0.0]),
        }
    }
}

#[derive(Debug, Inspectable)]
pub struct Health {
    #[export]
    pub hp: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self { hp: 100.0 }
    }
}

impl Component for Health {}

pub struct PlayerController;

impl Behaviour for PlayerController {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut input_dir: [f32; 2] = [0.0, 0.0];
        if engine.input.is_key_down(Keycode::W) {
            input_dir[1] += 1.0;
        }
        if engine.input.is_key_down(Keycode::S) {
            input_dir[1] -= 1.0;
        }
        if engine.input.is_key_down(Keycode::D) {
            input_dir[0] += 1.0;
        }
        if engine.input.is_key_down(Keycode::A) {
            input_dir[0] -= 1.0;
        }

        let len = (input_dir[0].powi(2) + input_dir[1].powi(2)).sqrt();
        if len > 0.0 {
            input_dir[0] /= len;
            input_dir[1] /= len;
        }

        let collisions = engine.collision_events.clone();
        let mut name_map = std::collections::HashMap::new();
        for ev in &collisions {
            for id in [ev.a, ev.b] {
                if !name_map.contains_key(&id) {
                    if let Some(name) = engine.get_entity_name(id) {
                        name_map.insert(id, name.to_string());
                    }
                }
            }
        }

        for (mut actor, player, transform, _velocity) in engine
            .stage()
            .query3_mut::<CustomPlayer, Transform, Velocity>()
        {
            let _ent = actor.entity();
            let acc = player.acceleration.unwrap_or(10.0);
            let damping = player.damping.unwrap_or(0.9);
            let max_speed = player.speed;
            let velocity = player.velocity.get_or_insert([0.0, 0.0]);

            let target = [input_dir[0] * max_speed, input_dir[1] * max_speed];
            velocity[0] += (target[0] - velocity[0]) * acc * delta;
            velocity[1] += (target[1] - velocity[1]) * acc * delta;
            velocity[0] *= damping;
            velocity[1] *= damping;

            transform.position[0] += velocity[0] * delta;
            transform.position[1] += velocity[1] * delta;

            for ev in &collisions {
                if ev.a == _ent || ev.b == _ent {
                    let other = if ev.a == _ent { ev.b } else { ev.a };
                    if let Some(name) = name_map.get(&other) {
                        println!("Player collided with {}", name);
                    }
                }
            }
        }
    }
}

struct MovementSystem;
impl Behaviour for MovementSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        for (mut actor, transform, vel) in engine.stage().query2_mut::<Transform, Velocity>() {
            let _e = actor.entity();
            transform.position[0] += vel.velocity[0] * delta;
            transform.position[1] += vel.velocity[1] * delta;
            transform.position[2] += vel.velocity[2] * delta;
        }
    }
}

fn forward_from_orientation(ori: [f32; 4]) -> [f32; 3] {
    let angle = 2.0 * f32::atan2(ori[2], ori[3]);
    [angle.cos(), angle.sin(), 0.0]
}

struct GameBehaviour {
    player: Entity,
    mouse_down: bool,
    spawn_timer: f32,
    score: u32,
    score_label: Option<Entity>,
}

impl GameBehaviour {
    fn update_score_label(&self, engine: &mut Engine) {
        if let Some(ent) = self.score_label {
            if let Some(label) = engine.world.get_mut::<UILabel>(ent) {
                label.text = format!("Score: {}", self.score);
                engine.egui_ctx.request_repaint();
            }
        }
    }
    fn spawn_bullet(&self, engine: &mut Engine) {
        if let Some(p_trans) = engine.world.get::<Transform>(self.player) {
            let position = p_trans.position;
            let orientation = p_trans.orientation;
            let dir = forward_from_orientation(orientation);
            if let Ok(prefab) = Prefab::load("assets/bullet.json") {
                if let Some(mut actor) = engine.instantiate_prefab(prefab) {
                    if let Some(t) = actor.get_component_mut::<Transform>() {
                        t.position = position;
                        t.orientation = orientation;
                    }
                    if let Some(vel) = actor.get_component_mut::<Velocity>() {
                        vel.velocity = [dir[0] * 15.0, dir[1] * 15.0, 0.0];
                    }
                }
            }
        }
    }

    fn spawn_enemy_at(&self, engine: &mut Engine, pos: [f32; 3]) {
        let mut enemy_obj = Object::default();
        enemy_obj.position = pos;
        enemy_obj.size = [0.5, 0.5, 0.5];
        enemy_obj.color = [255.0, 0.0, 0.0];
        enemy_obj.is_static = false;
        let next_index = engine.scene.objects.len();
        if let Some(mut actor) = engine.spawn_object_as_actor(enemy_obj) {
            let eid = next_index;
            if let Some(meta) = actor.get_component_mut::<Metadata>() {
                meta.name = format!("Enemy{}", eid);
                meta.tags.push("enemy".into());
            }
            actor.with_bundle((Velocity::default(), Collider::default()));
            if let Some(col) = actor.get_component_mut::<Collider>() {
                col.radius = 0.5;
                col.is_cube = true;
            }
        }
    }
}

impl Behaviour for GameBehaviour {
    fn start(&mut self, engine: &mut Engine) {
        self.spawn_enemy_at(engine, [3.0, 0.0, 0.0]);
        self.spawn_timer = 0.0;
        self.score = 0;
        {
            let mut stage = engine.stage();
            let mut label = stage.spawn_actor("scoreLabel");
            label.add_component::<Metadata>();
            if let Some(meta) = label.get_component_mut::<Metadata>() {
                meta.name = "scoreLabel".to_string();
            }
            label.add_component::<UIScreenSpace>();
            label.add_component::<UILabel>();
            if let Some(ui) = label.get_component_mut::<UILabel>() {
                ui.text = "Score: 0".to_string();
                ui.font_name = Some("monospace".into());
            }
            label.add_component::<UILayout>();
            if let Some(layout) = label.get_component_mut::<UILayout>() {
                layout.anchor = Anchor::TopLeft;
                layout.offset = [10.0, 10.0];
            }
            self.score_label = Some(label.entity());
        }
    }

    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let pressed = engine.input.is_mouse_button_down(MouseButton::Left);
        if pressed && !self.mouse_down {
            self.spawn_bullet(engine);
        }
        self.mouse_down = pressed;

        // update bullet lifetimes (first collect entities to delete)
        let mut to_delete = Vec::new();
        for (actor, life, _meta) in engine.stage().query2_mut::<Lifetime, Metadata>() {
            let e = actor.entity();
            life.remaining -= delta;
            if life.remaining <= 0.0 {
                to_delete.push(e);
            }
        }
        for e in to_delete {
            engine.delete_entity(e);
        }

        // spawn enemies periodically
        self.spawn_timer += delta;
        if self.spawn_timer >= 2.0 {
            self.spawn_timer = 0.0;
            let mut rng = rand::thread_rng();
            let x = rng.gen_range(-10.0..10.0);
            let y = rng.gen_range(-10.0..10.0);
            self.spawn_enemy_at(engine, [x, y, 0.0]);
        }

        let player_pos = engine
            .world
            .get::<Transform>(self.player)
            .map(|t| t.position)
            .unwrap_or([0.0; 3]);

        for (actor, t, v, meta) in engine.stage().query3_mut::<Transform, Velocity, Metadata>() {
            let _e = actor.entity();
            if meta.tags.iter().any(|tag| tag == "enemy") {
                let dx = player_pos[0] - t.position[0];
                let dy = player_pos[1] - t.position[1];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 0.0 {
                    v.velocity[0] = dx / dist * 2.0;
                    v.velocity[1] = dy / dist * 2.0;
                }
            }
        }

        let mut delete_collided = Vec::new();

        for ev in engine.collision_events.clone() {
            let a_enemy = engine.entity_has_tag(ev.a, "enemy");
            let b_enemy = engine.entity_has_tag(ev.b, "enemy");
            let a_bullet = engine.entity_has_tag(ev.a, "bullet");
            let b_bullet = engine.entity_has_tag(ev.b, "bullet");

            if (a_bullet && b_enemy) || (b_bullet && a_enemy) {
                let (bullet, enemy) = if a_bullet { (ev.a, ev.b) } else { (ev.b, ev.a) };
                if let Some(mut actor) = engine.stage().get_actor(bullet) {
                    actor.send_event("hit", enemy);
                }
                delete_collided.push(ev.a);
                delete_collided.push(ev.b);
                self.score += 1;
                self.update_score_label(engine);
            } else if (a_enemy && ev.b == self.player) || (b_enemy && ev.a == self.player) {
                if let Some(h) = engine.get_component_mut_entity::<Health>(self.player) {
                    h.hp -= 1.0;
                    println!("{:?}", h.hp);
                }
            }
        }

        for e in delete_collided {
            engine.delete_entity(e);
        }

        if let Some(h) = engine.world.get::<Health>(self.player) {
            if h.hp <= 0.0 {
                let entities = engine.world.entities().to_vec();
                for e in entities {
                    if engine.entity_has_tag(e, "enemy") || engine.entity_has_tag(e, "bullet") {
                        engine.delete_entity(e);
                    }
                }
                if let Some(hp) = engine.get_component_mut_entity::<Health>(self.player) {
                    hp.hp = 100.0;
                }
                self.spawn_timer = 0.0;
                self.score = 0;
                if let Some(ent) = self.score_label {
                    if let Some(label) = engine.world.get_mut::<UILabel>(ent) {
                        label.text = "Score: 0".to_string();
                    }
                }
            }
        }

        self.update_score_label(engine);
    }
}

struct TopDownShooterApp;

impl App for TopDownShooterApp {
    fn setup(&mut self, engine: &mut Engine) {
        engine.auto_register_component::<Health>("Health");
        engine.auto_register_component::<CustomPlayer>("Custom Player");
        engine.auto_register_component::<UILabel>("UI Label");
        engine.auto_register_component::<UILayout>("UI Layout");
        engine.auto_register_component::<UIScreenSpace>("UI Screen Space");

        let mut player_obj = Object::default();
        player_obj.is_static = false;
        player_obj.size = [0.5, 0.5, 0.5];
        player_obj.color = [0.0, 150.0, 255.0];
        let mut player_actor = engine.spawn_object_as_actor(player_obj).unwrap();
        if let Some(meta) = player_actor.get_component_mut::<Metadata>() {
            meta.name = "Player".into();
            meta.tags.push("player".into());
        }
        player_actor.with_bundle((Player, Velocity::default(), Collider::default()));
        if let Some(col) = player_actor.get_component_mut::<Collider>() {
            col.radius = 0.5;
            col.is_cube = true;
        }
        player_actor.with_bundle((
            LookAt::default(),
            Health::default(),
            CustomPlayer::default(),
        ));
        if let Some(look) = player_actor.get_component_mut::<LookAt>() {
            look.target = "mouse".into();
            look.rotate_x = true;
            look.rotate_y = true;
            look.rotate_z = true;
        }
        let player_ent = player_actor.entity();
        engine.add_behaviour(MovementSystem);
        engine.add_behaviour(PlayerController);
        engine.add_behaviour(LookAtBehaviour);
        engine.add_behaviour(GameBehaviour {
            player: player_ent,
            mouse_down: false,
            spawn_timer: 0.0,
            score: 0,
            score_label: None,
        });
    }

    fn update(&mut self, _engine: &mut Engine, _delta: f32) {}

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app().with_title("Top Down Shooter").run(TopDownShooterApp)
}
