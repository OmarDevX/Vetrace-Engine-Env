//! Feature-gated top-down 2D physics example.
//!
//! Run from the workspace root:
//! `cargo run -p vetrace_engine --example top_down_2d --features render_egui,render_2d,physics_2d`
//!
//! Controls: WASD/arrow keys move, mouse aims, left click shoots, R restarts.

use std::collections::BTreeSet;

use glam::{Quat, Vec2, Vec3, Vec4};
use vetrace_engine::prelude::*;
use vetrace_engine::render::RenderSettings;
use vetrace_engine::{DebugTextOverlayPanel, InputState};

const PLAYER_LAYER: u32 = 1 << 0;
const ZOMBIE_LAYER: u32 = 1 << 1;
const BULLET_LAYER: u32 = 1 << 2;
const PLAYER_CONTACT_DAMAGE_INTERVAL: f32 = 1.25;

#[derive(Clone, Copy, Debug)]
struct Player;

#[derive(Clone, Copy, Debug)]
struct Zombie;

#[derive(Clone, Copy, Debug)]
struct Bullet {
    ttl: f32,
}

struct TopDownGame {
    player: Entity,
    crosshair: Entity,
    zombies: Vec<Entity>,
    bullets: Vec<Entity>,
    touching_zombies: BTreeSet<Entity>,
    round: u32,
    score: u32,
    health: i32,
    round_delay: f32,
    game_over_delay: f32,
    hurt_cooldown: f32,
    rng: u64,
}

impl TopDownGame {
    fn new() -> Self {
        Self {
            player: Entity::INVALID,
            crosshair: Entity::INVALID,
            zombies: Vec::new(),
            bullets: Vec::new(),
            touching_zombies: BTreeSet::new(),
            round: 1,
            score: 0,
            health: 5,
            round_delay: 0.0,
            game_over_delay: 0.0,
            hurt_cooldown: 0.0,
            rng: 0xA53C_91E2_D4B7_8F01,
        }
    }

    fn spawn_player(&mut self, engine: &mut Engine) {
        self.player = engine
            .spawn_actor("Player")
            .with(Player)
            .with(Transform::default())
            .with(Sprite2D {
                size: Vec2::splat(0.9),
                tint: Vec4::new(0.15, 0.72, 1.0, 1.0),
                pixel_snap: true,
                ..Sprite2D::default()
            })
            .with(CanvasItem2D { z_index: 20, ..CanvasItem2D::default() })
            .with(RigidBody2D {
                mass: 2.0,
                gravity_scale: 0.0,
                linear_damping: 12.0,
                lock_rotation: true,
                ..RigidBody2D::dynamic()
            })
            .with(Collider2D {
                shape: ColliderShape2D::Circle,
                radius: 0.38,
                collision_layer: PLAYER_LAYER,
                collision_mask: ZOMBIE_LAYER,
                friction: 0.1,
                ..Collider2D::default()
            })
            .with(Velocity2D::default())
            .build()
            .entity();
    }

    fn spawn_crosshair(&mut self, engine: &mut Engine) {
        self.crosshair = engine
            .spawn_actor("Crosshair")
            .with(Transform::default())
            .with(Sprite2D {
                size: Vec2::splat(0.18),
                tint: Vec4::new(1.0, 0.95, 0.35, 0.9),
                ..Sprite2D::default()
            })
            .with(CanvasItem2D { z_index: 100, ..CanvasItem2D::default() })
            .build()
            .entity();
    }

    fn spawn_round(&mut self, engine: &mut Engine) {
        let player_position = entity_position(engine, self.player);
        let count = 3 + (self.round.saturating_sub(1) * 2) as usize;
        println!("Round {}: spawning {} zombies", self.round, count);
        for index in 0..count {
            let angle = self.random01() * std::f32::consts::TAU;
            let radius = 7.0 + self.random01() * 3.0;
            let position = player_position + Vec2::new(angle.cos(), angle.sin()) * radius;
            let zombie = engine
                .spawn_actor(format!("Zombie R{} #{}", self.round, index + 1))
                .with(Zombie)
                .with(Transform {
                    translation: position.extend(0.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                })
                .with(Sprite2D {
                    size: Vec2::splat(0.82),
                    tint: Vec4::new(0.38, 0.92, 0.30, 1.0),
                    ..Sprite2D::default()
                })
                .with(CanvasItem2D { z_index: 10, ..CanvasItem2D::default() })
                .with(RigidBody2D {
                    mass: 1.0,
                    gravity_scale: 0.0,
                    linear_damping: 8.0,
                    lock_rotation: true,
                    ..RigidBody2D::dynamic()
                })
                .with(Collider2D {
                    shape: ColliderShape2D::Circle,
                    radius: 0.36,
                    collision_layer: ZOMBIE_LAYER,
                    collision_mask: PLAYER_LAYER | ZOMBIE_LAYER | BULLET_LAYER,
                    friction: 0.0,
                    ..Collider2D::default()
                })
                .with(Velocity2D::default())
                .build()
                .entity();
            self.zombies.push(zombie);
        }
    }

    fn shoot(&mut self, engine: &mut Engine, target: Vec2) {
        let origin = entity_position(engine, self.player);
        let direction = (target - origin).normalize_or_zero();
        if direction == Vec2::ZERO { return; }
        let bullet_position = origin + direction * 0.62;
        let bullet = engine
            .spawn_actor("Bullet")
            .with(Bullet { ttl: 2.0 })
            .with(Transform {
                translation: bullet_position.extend(0.0),
                rotation: Quat::from_rotation_z(direction.y.atan2(direction.x)),
                scale: Vec3::ONE,
            })
            .with(Sprite2D {
                size: Vec2::new(0.38, 0.14),
                tint: Vec4::new(1.0, 0.68, 0.16, 1.0),
                ..Sprite2D::default()
            })
            .with(CanvasItem2D { z_index: 15, ..CanvasItem2D::default() })
            .with(RigidBody2D {
                mass: 0.1,
                gravity_scale: 0.0,
                lock_rotation: true,
                continuous: true,
                ..RigidBody2D::dynamic()
            })
            .with(Collider2D {
                shape: ColliderShape2D::Circle,
                radius: 0.11,
                sensor: true,
                collision_layer: BULLET_LAYER,
                collision_mask: ZOMBIE_LAYER,
                ..Collider2D::default()
            })
            .with(Velocity2D { linear: direction * 18.0, angular: 0.0 })
            .build()
            .entity();
        self.bullets.push(bullet);
    }

    fn process_collisions(&mut self, engine: &mut Engine) {
        let started = engine.drain_events::<CollisionStarted2D>();
        let contacts = engine.drain_events::<CollisionContact2D>();
        let stopped = engine.drain_events::<CollisionStopped2D>();
        let mut despawn = BTreeSet::new();

        for event in started {
            if let Some((bullet, zombie)) =
                match_pair(&self.bullets, &self.zombies, event.entity_a, event.entity_b)
            {
                if !despawn.contains(&bullet)
                    && !despawn.contains(&zombie)
                    && engine.raw_world().is_alive(bullet)
                    && engine.raw_world().is_alive(zombie)
                {
                    despawn.insert(bullet);
                    despawn.insert(zombie);
                    self.score = self.score.saturating_add(10);
                }
                continue;
            }

            if let Some(zombie) =
                zombie_in_player_pair(self.player, &self.zombies, event.entity_a, event.entity_b)
            {
                self.touching_zombies.insert(zombie);
            }
        }

        // Contact events are emitted every active physics step. They keep the
        // overlap state correct even while neither collider separates.
        for event in contacts {
            if let Some(zombie) =
                zombie_in_player_pair(self.player, &self.zombies, event.entity_a, event.entity_b)
            {
                self.touching_zombies.insert(zombie);
            }
        }

        for event in stopped {
            if let Some(zombie) =
                zombie_in_player_pair(self.player, &self.zombies, event.entity_a, event.entity_b)
            {
                self.touching_zombies.remove(&zombie);
            }
        }

        for entity in despawn {
            self.touching_zombies.remove(&entity);
            despawn_entity(engine, entity);
        }
        self.touching_zombies
            .retain(|entity| engine.raw_world().is_alive(*entity));

        if self.touching_zombies.is_empty() {
            // A later impact should deal damage immediately and start a fresh timer.
            self.hurt_cooldown = 0.0;
        } else if self.game_over_delay <= 0.0 && self.hurt_cooldown <= 0.0 {
            self.health -= 1;
            self.hurt_cooldown = PLAYER_CONTACT_DAMAGE_INTERVAL;
            if self.health <= 0 {
                self.game_over_delay = 1.5;
                println!("Game over. Restarting...");
            }
        }
    }

    fn update_player(&self, engine: &mut Engine, input: &InputState) {
        if self.game_over_delay > 0.0 { return; }
        let horizontal = axis(input, "A", "D", "ArrowLeft", "ArrowRight");
        let vertical = axis(input, "S", "W", "ArrowDown", "ArrowUp");
        let movement = Vec2::new(horizontal, vertical).normalize_or_zero() * 5.2;
        if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity2D>(self.player) {
            velocity.linear = movement;
        }
    }

    fn update_zombies(&self, engine: &mut Engine) {
        if self.game_over_delay > 0.0 { return; }
        let player_position = entity_position(engine, self.player);
        let speed = 2.05 + self.round as f32 * 0.08;
        let updates = self
            .zombies
            .iter()
            .copied()
            .filter(|entity| engine.raw_world().is_alive(*entity))
            .map(|entity| {
                let direction = (player_position - entity_position(engine, entity)).normalize_or_zero();
                (entity, direction * speed)
            })
            .collect::<Vec<_>>();
        for (entity, velocity_value) in updates {
            if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity2D>(entity) {
                velocity.linear = velocity_value;
            }
        }
    }

    fn update_bullets(&mut self, engine: &mut Engine, dt: f32) {
        let mut expired = Vec::new();
        for entity in self.bullets.iter().copied() {
            let Some(bullet) = engine.raw_world_mut().get_mut::<Bullet>(entity) else { continue; };
            bullet.ttl -= dt;
            if bullet.ttl <= 0.0 { expired.push(entity); }
        }
        for entity in expired { despawn_entity(engine, entity); }
    }

    fn restart(&mut self, engine: &mut Engine) {
        let mut old_entities = self.zombies.drain(..).collect::<Vec<_>>();
        old_entities.extend(self.bullets.drain(..));
        for entity in old_entities { despawn_entity(engine, entity); }
        if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(self.player) {
            *transform = Transform::default();
        }
        if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity2D>(self.player) {
            *velocity = Velocity2D::default();
        }
        self.touching_zombies.clear();
        self.round = 1;
        self.score = 0;
        self.health = 5;
        self.round_delay = 0.0;
        self.game_over_delay = 0.0;
        self.hurt_cooldown = 0.0;
        self.spawn_round(engine);
    }

    fn random01(&mut self) -> f32 {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.rng >> 40) as u32 as f32) / ((1u32 << 24) - 1) as f32
    }
}

impl App for TopDownGame {
    fn setup(&mut self, engine: &mut Engine) {
        engine.insert_resource(Camera2D {
            pixels_per_unit: 48.0,
            zoom: 1.0,
            pixel_snap: false,
            ..Camera2D::default()
        });
        let mut physics_2d = Physics2dState::default();
        physics_2d.gravity = Vec2::ZERO;
        physics_2d.solver_iterations = 5;
        physics_2d.max_substeps = 8;
        physics_2d.broadphase_cell_size = 1.5;
        engine.insert_resource(physics_2d);
        engine.insert_resource(DebugTextOverlayPanel::default());

        engine
            .spawn_actor("Arena Background")
            .with(Transform::default())
            .with(Sprite2D {
                size: Vec2::new(80.0, 80.0),
                tint: Vec4::new(0.035, 0.055, 0.075, 1.0),
                ..Sprite2D::default()
            })
            .with(CanvasItem2D { z_index: -100, ..CanvasItem2D::default() })
            .build();

        self.spawn_player(engine);
        self.spawn_crosshair(engine);
        self.spawn_round(engine);
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
        if input.quit_requested() || input.was_key_pressed("Escape") {
            engine.stop();
            return;
        }

        self.hurt_cooldown = (self.hurt_cooldown - dt).max(0.0);
        self.process_collisions(engine);
        self.zombies.retain(|entity| engine.raw_world().is_alive(*entity));
        self.bullets.retain(|entity| engine.raw_world().is_alive(*entity));

        if input.was_key_pressed("R") { self.restart(engine); }

        if self.game_over_delay > 0.0 {
            self.game_over_delay -= dt;
            if self.game_over_delay <= 0.0 { self.restart(engine); }
        } else if self.zombies.is_empty() {
            if self.round_delay <= 0.0 { self.round_delay = 1.0; }
            self.round_delay -= dt;
            if self.round_delay <= 0.0 {
                self.round = self.round.saturating_add(1);
                self.spawn_round(engine);
            }
        }

        self.update_player(engine, &input);
        self.update_zombies(engine);
        self.update_bullets(engine, dt);

        let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
        let player_position = entity_position(engine, self.player);
        let mut input_camera = engine.get_resource::<Camera2D>().cloned().unwrap_or_default();
        input_camera.position = player_position;
        let mouse = Vec2::new(input.mouse_position().0, input.mouse_position().1);
        let mouse_world = input_camera.screen_to_world(
            mouse,
            Vec2::new(settings.width as f32, settings.height as f32),
        );
        if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(self.crosshair) {
            transform.translation = mouse_world.extend(0.0);
        }
        let aim = (mouse_world - player_position).normalize_or_zero();
        if aim != Vec2::ZERO {
            if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(self.player) {
                transform.rotation = Quat::from_rotation_z(aim.y.atan2(aim.x));
            }
        }
        if input.was_mouse_button_pressed("Left") && self.game_over_delay <= 0.0 {
            self.shoot(engine, mouse_world);
        }
        if let Some(camera) = engine.get_resource_mut::<Camera2D>() {
            camera.position = player_position;
        }

        if let Some(panel) = engine.get_resource_mut::<DebugTextOverlayPanel>() {
            panel.enabled = true;
            panel.title = "Top-down 2D Physics".to_owned();
            panel.subtitle = format!("Round {}", self.round);
            panel.status = if self.game_over_delay > 0.0 {
                "GAME OVER".to_owned()
            } else if self.round_delay > 0.0 {
                "Next round...".to_owned()
            } else {
                format!("{} zombies remain", self.zombies.len())
            };
            panel.lines = vec![
                format!("Health: {}/5", self.health.max(0)),
                format!("Score: {}", self.score),
                format!("Bullets: {}", self.bullets.len()),
            ];
            panel.controls = vec![
                "WASD / arrows: move".to_owned(),
                "Mouse: aim · Left click: shoot".to_owned(),
                "R: restart · Esc: quit".to_owned(),
            ];
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace — Top-down 2D Physics".to_owned(),
            width: 1280,
            height: 720,
            cursor_grab: false,
            cursor_visible: true,
            clear_color: [0.008, 0.012, 0.02, 1.0],
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .add_plugin(Render2dPlugin::new())
        .add_plugin(Physics2dPlugin::new())
        .run_until_stopped(TopDownGame::new(), None, 1.0 / 60.0)
}

fn entity_position(engine: &Engine, entity: Entity) -> Vec2 {
    engine
        .raw_world()
        .get::<Transform>(entity)
        .map(|transform| transform.translation.truncate())
        .unwrap_or(Vec2::ZERO)
}

fn despawn_entity(engine: &mut Engine, entity: Entity) {
    if let Some(actor) = engine.actor(entity) { let _ = actor.despawn(engine); }
}

fn axis(input: &InputState, negative: &str, positive: &str, alt_negative: &str, alt_positive: &str) -> f32 {
    let negative = input.is_key_down(negative) || input.is_key_down(alt_negative);
    let positive = input.is_key_down(positive) || input.is_key_down(alt_positive);
    positive as u8 as f32 - negative as u8 as f32
}

fn zombie_in_player_pair(
    player: Entity,
    zombies: &[Entity],
    entity_a: Entity,
    entity_b: Entity,
) -> Option<Entity> {
    if entity_a == player && zombies.contains(&entity_b) {
        Some(entity_b)
    } else if entity_b == player && zombies.contains(&entity_a) {
        Some(entity_a)
    } else {
        None
    }
}

fn match_pair(
    left_set: &[Entity],
    right_set: &[Entity],
    a: Entity,
    b: Entity,
) -> Option<(Entity, Entity)> {
    if left_set.contains(&a) && right_set.contains(&b) {
        Some((a, b))
    } else if left_set.contains(&b) && right_set.contains(&a) {
        Some((b, a))
    } else {
        None
    }
}
