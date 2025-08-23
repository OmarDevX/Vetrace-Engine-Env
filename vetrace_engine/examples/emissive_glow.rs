use glam::{Mat3, Quat, Vec3};
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

use std::collections::HashMap;
use vetrace_engine::components::components::FreeFlightControls;
use vetrace_editor::EditorPlugin;
use vetrace_engine::app::{app, App};
use vetrace_engine::components::components::{CameraAttachment, Metadata, Transform};
use vetrace_engine::ecs::{Component, Entity};
use vetrace_engine::engine::Engine;
use vetrace_engine::inspector::Inspectable;
use vetrace_engine::scene::object::Object;
use vetrace_engine::Behaviour;
use vetrace_engine_macros::Inspectable;
use vetrace_engine::inspector::export::{ExportedField,ExportKind};
use vetrace_engine::components::components::LookAt;
// ================== Tunables ==================
const GRAVITY_STRENGTH: f32 = 80.0;
const WALK_SPEED: f32      = 6.0;
const ACCEL_GROUND: f32    = 18.0;
const ACCEL_AIR: f32       = 4.0;
const GROUND_DRAG: f32     = 12.0;
const MAX_FALL_SPEED: f32  = 40.0;
const EXTRA_DOWNFORCE: f32 = 25.0;
const JUMP_IMPULSE: f32    = 7.5;
const JUMP_COOLDOWN: f32   = 0.18;
const PLAYER_RADIUS: f32   = 0.5;
const GROUND_SNAP: f32     = 0.05;

const PLANET_ROT_SPEED: f32 = 0.1;
// ==============================================

#[derive(Debug, Clone, Inspectable)]
pub struct PlanetWalker {
    #[export] pub walk_speed: f32,
    #[export] pub grounded: bool,
    #[export] pub surface_normal: [f32; 3],
    #[export] pub jump_cooldown: f32,
}
impl Component for PlanetWalker {}
impl Default for PlanetWalker {
    fn default() -> Self {
        Self { walk_speed: WALK_SPEED, grounded: false, surface_normal: [0.0, 1.0, 0.0], jump_cooldown: 0.0 }
    }
}

#[derive(Debug, Clone, Inspectable)]
pub struct KVel { #[export] pub v: [f32; 3] }
impl Default for KVel { fn default() -> Self { Self { v: [0.0, 0.0, 0.0] } } }
impl Component for KVel {}

#[derive(Debug, Inspectable)]
pub struct Planet {
    #[export] pub radius: f32,
    #[export] pub mass: f32,
    #[export] pub rotation_speed: f32,
}
impl Component for Planet {}
impl Default for Planet {
    fn default() -> Self { Self { radius: 50.0, mass: 1000.0, rotation_speed: PLANET_ROT_SPEED } }
}

// ================= Helpers ====================
fn nearest_planet(pos: Vec3, planets: &[(Vec3, f32)]) -> (Vec3, f32) {
    let mut best = (Vec3::ZERO, 1.0f32, f32::INFINITY);
    for (c, r) in planets {
        let gap = (pos - *c).length() - (*r + PLAYER_RADIUS);
        if gap < best.2 { best = (*c, *r, gap); }
    }
    (best.0, best.1)
}
fn project_tangent_normalize(v: Vec3, n: Vec3) -> Vec3 {
    let t = v - v.dot(n) * n;
    let l = t.length();
    if l > 1e-6 { t / l } else {
        let hint = if n.dot(Vec3::Y).abs() < 0.99 { Vec3::Y } else { Vec3::Z };
        (n.cross(hint)).normalize()
    }
}
// ==============================================

// ---------- Visual spin for planets (no center move) ----------
struct PlanetRotationSystem;
impl Behaviour for PlanetRotationSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        for (_a, p, t) in engine.stage().query2_mut::<Planet, Transform>() {
            let rot = Quat::from_rotation_y(p.rotation_speed * delta);
            let q = Quat::from_xyzw(t.orientation[0], t.orientation[1], t.orientation[2], t.orientation[3]);
            let nq = (rot * q).normalize();
            t.orientation = [nq.x, nq.y, nq.z, nq.w];
        }
    }
}


// ---------- Kinematic spherical walker ----------
struct SimpleSphericalPhysicsSystem;
impl Behaviour for SimpleSphericalPhysicsSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let dt = delta.clamp(1e-4, 1.0 / 20.0);

        // Snapshot input BEFORE borrowing stage mutably
        let w = engine.input.is_key_down(Keycode::W);
        let s = engine.input.is_key_down(Keycode::S);
        let a = engine.input.is_key_down(Keycode::A);
        let d = engine.input.is_key_down(Keycode::D);
        let space = engine.input.is_key_down(Keycode::Space);

        // Planets (immutable)
        let planets: Vec<(Vec3, f32)> = {
            let mut stage = engine.stage();
            let mut v = Vec::new();
            for (_actor, p, t) in stage.query2::<Planet, Transform>() {
                v.push((Vec3::from_array(t.position), p.radius));
            }
            v
        };

        // Snapshot active camera forward (world), independent of player
        let cam_forward_world: Option<Vec3> = {
            let mut stage = engine.stage();
            let mut res = None;
            for (_actor, _att, t) in stage.query2::<CameraAttachment, Transform>() {
                let q = Quat::from_xyzw(t.orientation[0], t.orientation[1], t.orientation[2], t.orientation[3]);
                // In your FreeFlight basis: forward = q * X, up = q * Y
                res = Some((q * Vec3::X).normalize());
                break;
            }
            res
        };

        // Update walkers
        let mut stage = engine.stage();
        for (actor, walker, t, kv) in stage.query3_mut::<PlanetWalker, Transform, KVel>() {
            let mut pos = Vec3::from_array(t.position);
            let mut v = Vec3::from_array(kv.v);

            let (center, pr) = nearest_planet(pos, &planets);
            let to_center = pos - center;
            let dist = to_center.length().max(1e-6);
            let up = to_center / dist;

            let desired_r = pr + PLAYER_RADIUS;
            let surf_gap = dist - desired_r;
            let grounded = surf_gap <= GROUND_SNAP;

            walker.grounded = grounded;
            walker.surface_normal = up.to_array();
            walker.jump_cooldown = (walker.jump_cooldown - dt).max(0.0);

            // Heading from camera (independent cam)
            let heading_forward = cam_forward_world
                .map(|f| project_tangent_normalize(f, up))
                .unwrap_or_else(|| project_tangent_normalize(Vec3::Z, up));
            let heading_right = heading_forward.cross(up).normalize();

            // Input dir (tangent)
            let mut in_dir = Vec3::ZERO;
            if w { in_dir += heading_forward; }
            if s { in_dir -= heading_forward; }
            if a { in_dir -= heading_right; }
            if d { in_dir += heading_right; }
            if in_dir.length_squared() > 0.0 { in_dir = project_tangent_normalize(in_dir, up); }

            // Decompose velocity
            let v_radial = up * v.dot(up);
            let mut v_tan = v - v_radial;

            // Gravity
            let g = (-up) * GRAVITY_STRENGTH;
            let mut new_v_rad = v_radial + g * dt;

            // Inward cap
            let inward = -new_v_rad.dot(up);
            if inward > MAX_FALL_SPEED { new_v_rad += up * (inward - MAX_FALL_SPEED); }

            // Ground contact
            if grounded {
                let outward = new_v_rad.dot(up).max(0.0);
                if outward > 0.0 { new_v_rad -= up * outward; }
                new_v_rad += (-up) * (EXTRA_DOWNFORCE * dt);

                let pred = pos + (new_v_rad + v_tan) * dt;
                let nd = (pred - center).length();
                if nd < desired_r || surf_gap < 0.0 || surf_gap <= GROUND_SNAP {
                    pos = center + up * desired_r;
                    new_v_rad = Vec3::ZERO;
                }
            }

            // Steering
            let target_speed = walker.walk_speed;
            let desired_tan = if in_dir == Vec3::ZERO { Vec3::ZERO } else { in_dir * target_speed };
            let rate = if grounded { ACCEL_GROUND } else { ACCEL_AIR };
            let alpha = (rate * dt).clamp(0.0, 1.0);
            v_tan = v_tan + (desired_tan - v_tan) * alpha;

            if grounded && in_dir == Vec3::ZERO {
                let drag = (GROUND_DRAG * dt).clamp(0.0, 1.0);
                v_tan *= 1.0 - drag;
            }

            let sp = v_tan.length();
            if sp > target_speed { v_tan = v_tan / sp * target_speed; }

            // Jump
            if grounded && space && walker.jump_cooldown <= 0.0 {
                new_v_rad += up * JUMP_IMPULSE;
                walker.jump_cooldown = JUMP_COOLDOWN;
            }

            v = v_tan + new_v_rad;
            pos += v * dt;

            // Prevent tunneling
            let to_c2 = pos - center;
            let d2 = to_c2.length().max(1e-6);
            if d2 < desired_r {
                pos = center + to_c2 / d2 * desired_r;
                v -= up * v.dot(up).min(0.0);
            }

            // Orient player to heading (visual only)
            let right = heading_forward.cross(up).normalize();
            let forward = heading_forward;
            let basis = Mat3::from_cols(right, up, forward);
            let q = Quat::from_mat3(&basis);

            t.position = pos.to_array();
            t.orientation = [q.x, q.y, q.z, q.w];
            kv.v = v.to_array();
        }
    }
}

// ================= Game bootstrap =================
struct PlanetWalkingGame { player_entity: Entity, camera_entity: Entity }
impl Default for PlanetWalkingGame {
    fn default() -> Self { Self { player_entity: Entity(0), camera_entity: Entity(0) } }
}

impl App for PlanetWalkingGame {
    fn setup(&mut self, engine: &mut Engine) {
        // Register components
            // Load the cat model
 let assets = engine.assets.clone();
        let cat_id = assets.load_gltf_pbr(engine, "munchkin_cat/scene.gltf").expect("s");
        
        engine.auto_register_component::<PlanetWalker>("Planet Walker");
        engine.auto_register_component::<Planet>("Planet");
        engine.auto_register_component::<KVel>("Kinematic Velocity");
        engine.auto_register_component::<FreeFlightControls>("FreeFlight Controls");
        engine.auto_register_component::<LookAt>("LookAt");

        // Planets
        let planet_configs = [
            (Vec3::new(0.0, 0.0, 0.0), 50.0, [0.2, 0.7, 0.3]),
            (Vec3::new(150.0, 50.0, 0.0), 30.0, [0.7, 0.3, 0.8]),
            (Vec3::new(-120.0, -30.0, 80.0), 35.0, [0.8, 0.5, 0.2]),
        ];
        for (pos, radius, color) in planet_configs {
            let mut obj = Object::default();
            obj.is_cube = false;
            obj.radius = radius;
            obj.position = pos.to_array();
            obj.color = color;
            obj.is_static = true;

            if let Some(mut actor) = engine.spawn_object_as_actor(obj) {
                if let Some(meta) = actor.get_component_mut::<Metadata>() {
                    meta.name = format!("Planet_{}", pos.x as i32);
                }
                let e = actor.entity();
                engine.world.insert(e, Planet { radius, ..Default::default() });
            }
        }

        // Player (kinematic sphere)
        let mut player_obj = Object::default();
        player_obj.is_cube = false;
        player_obj.radius = PLAYER_RADIUS;
        player_obj.position = [0.0, 50.0 + PLAYER_RADIUS + 0.02, 0.0];
        player_obj.color = [1.0, 1.0, 1.0];
        player_obj.is_static = false;

        if let Some(mut pl) = engine.spawn_object_as_actor(player_obj) {
            if let Some(meta) = pl.get_component_mut::<Metadata>() {
                meta.name = "Player".to_string();
                meta.tags.push("player".to_string());
            }
            let e = pl.entity();
            engine.world.insert(e, PlanetWalker::default());
            engine.world.insert(e, KVel::default());
            self.player_entity = e;
        }

        // Independent camera entity with FreeFlightControls (RMB to move/rotate)
        let cam = engine.spawn_empty("camera");
        engine.world.insert(cam, Transform { position: [0.0, 75.0, -30.0], ..Default::default() });
        engine.world.insert(cam, CameraAttachment::default());
        engine.world.insert(cam, FreeFlightControls::default());
        self.camera_entity = cam;

        // Light
        let mut sun = Object::default();
        sun.is_cube = false;
        sun.radius = 100.0;
        sun.position = [200.0, 200.0, -300.0];
        sun.color = [1.0, 1.0, 0.8];
        sun.emission = 8.0;
        engine.spawn_object(sun);

        // Systems: camera (free flight) is fully independent
        engine.add_behaviour(SimpleSphericalPhysicsSystem);
        engine.add_behaviour(PlanetRotationSystem);
    }

    fn update(&mut self, _engine: &mut Engine, _delta: f32) {}
    fn render(&mut self, engine: &mut Engine) { engine.render_frame(); }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("Planet Walking — Independent FreeFlight Camera")
        .add_plugin(EditorPlugin::new())
        .run(PlanetWalkingGame::default())
}