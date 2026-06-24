use std::{collections::HashSet, rc::Rc, time::Instant};

use ahash::*;
use egui::{Context as EguiContext, Event, Pos2};
use sdl2::event::Event as SdlEvent;
use sdl2::mouse::MouseButton;

use crate::behaviour::component_lua::LuaComponentBehaviour;
use crate::behaviour::rotator::Rotator;
use crate::behaviour::script::EntityProxy;
use crate::behaviour::script::ScriptBehaviour;
use crate::components::components::CameraAttachment;
use crate::components::components::{
    AngularVelocity, Collider, GlobalTransform, LookAt, Material, ObjectRef, Player, Renderable,
    Rotate, ScriptComponent, Shape, Skin, Transform, Velocity,
};
use crate::components::generated::{
    FieldType, GeneratedComponent, GeneratedSpec, GeneratedStorage,
};
use crate::custom_material::{CustomMaterial, MaterialParameter};
use crate::ecs::Entity;
use crate::ecs::{Component, World};
use crate::engine::component_io::{apply_component_data, export_component_data};
use crate::engine::core::EngineCore;
use crate::events::{Event as CustomEvent, LuaEvent, SceneEvents};
use crate::input::{Input, window::WindowManager};
use crate::inspector::Inspectable;
use crate::math::{look_at, perspective, vec3_to_array};
#[cfg(feature = "use_epi")]
use crate::rendering::EguiRenderer;
use crate::rendering::RenderParams;
use crate::rendering::Renderer;
use crate::scene::factories::{player_factory, rotate_factory};
use crate::scene::object::Object;
use crate::scene::{
    loader::{ComponentFactory, ComponentFile, EntityFile, NodeFile, SceneFile, save_scene},
    scene::Scene,
};
use crate::systems::collision::CollisionEvent;
use crate::systems::free_flight::FreeFlightState;

use glam::{Mat3, Quat, Vec3};
use mlua::{Function, Lua, Value as LuaValue};
use rapier3d::prelude::*;
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
pub struct CameraInfo {
    pub position: Vec3,
    pub orientation: Quat,
    pub fov: f32,
    pub velocity: Vec3,
}

impl Default for CameraInfo {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            fov: 60.0_f32.to_radians(),
            velocity: Vec3::ZERO,
        }
    }
}
use crate::systems::animation::AnimationSystem;
use crate::systems::audio::AudioSystem;
use crate::systems::gizmo::GizmoSystem;
use crate::systems::selection::SelectionSystem;
// Note: MainWindow and SandboxWindow have been moved to vetrace_editor crate
use crate::Behaviour;

pub struct EmptyBehaviour;
impl Behaviour for EmptyBehaviour {}

pub fn sdl_event_to_egui_event(event: &SdlEvent, mouse_pos: Pos2) -> Option<Event> {
    use egui::{Event, PointerButton};
    match event {
        SdlEvent::MouseMotion { .. } => Some(Event::PointerMoved(mouse_pos)),
        SdlEvent::MouseButtonDown { mouse_btn, .. } => Some(Event::PointerButton {
            pos: mouse_pos,
            button: match mouse_btn {
                MouseButton::Left => PointerButton::Primary,
                MouseButton::Right => PointerButton::Secondary,
                MouseButton::Middle => PointerButton::Middle,
                _ => return None,
            },
            pressed: true,
            modifiers: Default::default(),
        }),
        SdlEvent::MouseButtonUp { mouse_btn, .. } => Some(Event::PointerButton {
            pos: mouse_pos,
            button: match mouse_btn {
                MouseButton::Left => PointerButton::Primary,
                MouseButton::Right => PointerButton::Secondary,
                MouseButton::Middle => PointerButton::Middle,
                _ => return None,
            },
            pressed: false,
            modifiers: Default::default(),
        }),
        SdlEvent::TextInput { text, .. } => Some(Event::Text(text.clone())),
        _ => None,
    }
}

pub struct Engine {
    pub core: EngineCore,
    pub world: World,
    pub renderer: Renderer,
    pub scene: Scene,
    pub physics: crate::engine::physics::PhysicsState,
    pub input: Input,
    pub window: WindowManager,
    pub running: bool,
    pub sky_color: [f32; 3],
    pub is_fisheye: bool,
    pub selection_mask: i32,
    // Note: sandbox_window moved to vetrace_editor crate
    pub sdl_context: sdl2::Sdl,
    pub egui_ctx: EguiContext,
    pub assets: std::sync::Arc<crate::assets::AssetManager>,
    #[cfg(feature = "use_epi")]
    pub egui_renderer: EguiRenderer,
    pub egui_events: Vec<Event>,
    pub behaviours: Vec<Box<dyn Behaviour>>,
    pub script_library: HashMap<String, ScriptBehaviour>,
    pub component_behaviours: HashMap<String, LuaComponentBehaviour>,
    pub component_factories: HashMap<String, ComponentFactory>,
    pub component_adders: HashMap<String, Rc<dyn Fn(&mut Engine, Entity)>>,
    pub component_removers: HashMap<String, Rc<dyn Fn(&mut Engine, Entity)>>,
    pub component_editors: HashMap<String, Rc<dyn Fn(&mut Engine, Entity, &mut egui::Ui)>>,
    pub component_checkers: HashMap<String, Rc<dyn Fn(&World, Entity) -> bool>>,
    pub component_accessors:
        HashMap<String, fn(&mut Engine, Entity) -> Option<&mut dyn Inspectable>>,
    pub generated_components: Vec<String>,
    pub generated_specs: HashMap<String, GeneratedSpec>,
    pub collision_events: Vec<CollisionEvent>,
    pub collision_event: CustomEvent<CollisionEvent>,
    pub entity_events: Vec<(Entity, Entity, String)>,
    pub entity_event: CustomEvent<(Entity, Entity, String)>,
    pub scene_events: SceneEvents,
    pub free_flight: FreeFlightState,
    // Note: main_window moved to vetrace_editor crate
    pub scene_manager: crate::engine::SceneManager,
    pub is_2d: bool,
    pub started_scripts: std::collections::HashSet<Entity>,
    pub paused: bool,
    pub saved_scene: Option<SceneFile>,
    pub ui_callbacks:
        Vec<Box<dyn FnMut(&egui::Context, &mut Engine) -> Result<(), Box<dyn std::error::Error>>>>,
    #[cfg(feature = "wgpu")]
    pub cached_gpu_materials: Vec<crate::scene::object::GpuMaterial>,
    #[cfg(feature = "wgpu")]
    pub cached_tex_handles: Vec<crate::gpu::TextureHandle>,
    #[cfg(feature = "wgpu")]
    pub cached_custom_materials: Vec<crate::scene::object::GpuCustomMaterial>,
    #[cfg(feature = "wgpu")]
    pub cached_custom_names: Vec<String>,
    #[cfg(feature = "wgpu")]
    pub cached_shader_defs: Vec<(String, String)>,
    #[cfg(feature = "wgpu")]
    pub materials_dirty: bool,
}

impl Engine {
    /// Obtain a [`World`] wrapper for high-level scene manipulation.
    pub fn world(&mut self) -> super::world::World<'_> {
        super::world::World::new(self)
    }
    /// Obtain a [`Stage`] wrapper returning [`Actor`]s from queries.
    pub fn stage(&mut self) -> super::stage::Stage<'_> {
        super::stage::Stage::new(self)
    }
    pub fn register_default_behaviours(&mut self) {
        self.add_behaviour(Rotator::new());
        self.add_behaviour(crate::systems::collision::CollisionSystem);
        self.add_behaviour(crate::systems::rapier_physics::RapierPhysicsSystem);
        self.add_behaviour(crate::systems::transform_sync::TransformSyncSystem);
        self.add_behaviour(crate::systems::hierarchy::HierarchySystem::default());
        self.add_behaviour(AudioSystem::new());
        self.add_behaviour(SelectionSystem::new());
        self.add_behaviour(GizmoSystem::new());
        self.add_behaviour(crate::systems::raycast::RaycastSystem);
        self.add_behaviour(crate::behaviour::post_processing::PostProcessBehaviour);
        self.add_behaviour(crate::systems::particle_cpu::CpuParticleSystem::default());
        self.add_behaviour(crate::systems::lerp::LerpSystem::default());
        self.add_behaviour(crate::systems::timer::TimerSystem::default());
        self.add_behaviour(AnimationSystem::new(self.assets.clone()));
        #[cfg(feature = "wgpu")]
        self.add_behaviour(crate::systems::sprite_mesh::SpriteMeshSystem::default());
    }

    pub fn ensure_generated_folder(&self) {
        let base = std::path::Path::new("generated");
        let _ = std::fs::create_dir_all(base.join("components"));
        let _ = std::fs::create_dir_all(base.join("behaviours"));
    }

    #[cfg(feature = "wgpu")]
    pub fn invalidate_material_cache(&mut self) {
        self.materials_dirty = true;
    }

    /// Attach a [`CustomMaterial`] to an entity and ensure GPU data is rebuilt.
    pub fn insert_custom_material(&mut self, entity: Entity, material: CustomMaterial) {
        self.world.insert(entity, material);
        #[cfg(feature = "wgpu")]
        {
            self.invalidate_material_cache();
        }
    }

    pub fn create_custom_component(&self, name: &str) {
        self.ensure_generated_folder();
        let path =
            std::path::Path::new("generated/behaviours").join(format!("{name}Behaviour.lua"));
        if !path.exists() {
            let template =
                "function start(engine, self) end\n\nfunction update(engine, self, delta) end\n";
            let _ = std::fs::write(path, template);
        }
    }

    pub fn generate_component_file(&self, name: &str, fields: &[(String, String, String)]) {
        self.ensure_generated_folder();
        let path = std::path::Path::new("generated/components").join(format!("{name}.rs"));
        let mut out = String::new();
        out.push_str("use crate::ecs::Component;\n");
        out.push_str(
            "use crate::inspector::{Inspectable, export::{ExportedField, ExportKind}};\n\n",
        );
        out.push_str(&format!(
            "#[derive(Default, Debug)]\npub struct {name} {{\n"
        ));
        for (fname, ftype, _) in fields {
            out.push_str(&format!("    pub {fname}: {ftype},\n"));
        }
        out.push_str("}\n");
        out.push_str(&format!("impl Component for {name} {{}}\n"));
        out.push_str(&format!("impl Inspectable for {name} {{\n    fn exported_fields_mut(&mut self) -> Vec<ExportedField> {{\n        vec![\n"));
        for (fname, ftype, _d) in fields {
            let kind = if *ftype == "bool" {
                "ExportKind::Checkbox"
            } else {
                "ExportKind::Slider { min: 0.0, max: 100.0 }"
            };
            out.push_str(&format!("            ExportedField {{ name: \"{fname}\", kind: {kind}, value: &mut self.{fname} as *mut _ as *mut dyn std::any::Any, type_id: std::any::TypeId::of::<{ftype}>(), }},\n"));
        }
        out.push_str("        ]\n    }\n}\n");
        let _ = std::fs::write(&path, out);
        let spec_path = path.with_extension("spec");
        let mut spec_out = String::new();
        for (fname, ftype, _d) in fields {
            spec_out.push_str(&format!("{fname} {ftype}\n"));
        }
        let _ = std::fs::write(spec_path, spec_out);
    }

    pub fn rename_entity(&mut self, entity: Entity, name: &str) {
        if let Some(meta) = self
            .world
            .get_mut::<crate::components::components::Metadata>(entity)
        {
            meta.name = name.to_string();
        }
    }

    /// Find an actor by name if it exists.
    pub fn find_actor_by_name(&mut self, name: &str) -> Option<super::Actor<'_>> {
        for &e in self.world.entities() {
            if let Some(meta) = self.world.get::<crate::components::components::Metadata>(e) {
                if meta.name == name {
                    return Some(super::Actor::new(self, e));
                }
            }
        }
        None
    }

    /// Find an entity by name if it exists.
    pub fn find_entity_by_name(&mut self, name: &str) -> Option<Entity> {
        self.find_actor_by_name(name).map(|a| a.entity())
    }
    /// Enable or disable mouse capture (relative mouse mode).
    pub fn capture_mouse(&mut self, capture: bool) {
        let mouse = self.sdl_context.mouse();
        let _ = mouse.set_relative_mouse_mode(capture);
        mouse.show_cursor(!capture);
        self.input.mouse_captured = capture;
    }
    pub fn duplicate_entity(&mut self, entity: Entity) -> Option<Entity> {
        let obj_id = self.world.get::<ObjectRef>(entity)?.id as usize;
        let object = *self.scene.objects.get(obj_id)?;
        self.spawn_object(object);
        let new_id = self.scene.objects.len() as u32 - 1;
        let new_entity = self.core.find_entity_by_object_id(new_id)?;
        if let (Some(src), Some(dst)) = (
            self.world
                .get::<crate::components::components::Metadata>(entity)
                .cloned(),
            self.world
                .get_mut::<crate::components::components::Metadata>(new_entity),
        ) {
            dst.name = format!("{} Copy", src.name);
            dst.tags = src.tags.clone();
        }
        let comps = self.list_components_entity(entity);
        for name in comps {
            if name == "ObjectRef" {
                continue;
            }
            let has_component = self
                .component_checkers
                .get(&name)
                .map(|check| check(&self.world, new_entity))
                .unwrap_or(false);
            if !has_component {
                if let Some(add) = self.component_adders.get(&name).cloned() {
                    add(self, new_entity);
                } else if self.generated_components.contains(&name) {
                    self.add_generated_component(new_entity, &name);
                }
            }
            if let Some(src_comp) = self.access_component_mut(entity, &name) {
                let data = export_component_data(src_comp);
                drop(src_comp);
                if let Some(dst_comp) = self.access_component_mut(new_entity, &name) {
                    apply_component_data(dst_comp, &data);
                }
            }
        }
        Some(new_entity)
    }

    pub fn active_camera_info(&self) -> CameraInfo {
        let mut info = CameraInfo::default();
        for (entity, transform, cam) in self.world.query2::<Transform, CameraAttachment>() {
            let gt = self
                .world
                .get::<crate::components::components::GlobalTransform>(entity)
                .cloned();
            let (pos, ori) = if let Some(g) = gt {
                (g.position, g.orientation)
            } else {
                (transform.position, transform.orientation)
            };
            let orient = Quat::from_xyzw(ori[0], ori[1], ori[2], ori[3]);
            info.orientation = orient;
            let offset = orient * Vec3::from(cam.local_offset);
            info.position = Vec3::from(pos) + offset;
            info.fov = cam.fov;
            if let Some(v) = self.world.get::<Velocity>(entity) {
                info.velocity = Vec3::from(v.velocity);
            }
            return info;
        }
        info
    }

    /// Shift all world and physics objects so the active camera is at the origin
    pub fn shift_origin(&mut self, offset: Vec3) {
        if offset.length_squared() == 0.0 {
            return;
        }

        for (_e, t) in self.world.query_mut::<Transform>() {
            let pos = Vec3::from_array(t.position) - offset;
            t.position = pos.to_array();
        }

        for (_e, gt) in self.world.query_mut::<GlobalTransform>() {
            let pos = Vec3::from_array(gt.position) - offset;
            gt.position = pos.to_array();
        }

        for (_handle, body) in self.physics.bodies.iter_mut() {
            let p = body.translation();
            body.set_translation(
                vector![p.x - offset.x, p.y - offset.y, p.z - offset.z],
                true,
            );
        }
    }

    /// Complete render method for app framework
    /// This replicates the full rendering pipeline from run.rs
    pub fn render_frame(&mut self) {
        use crate::math::{look_at, perspective, vec3_to_array};

        // Complete scene update pipeline (from run.rs line 135-141)
        self.update_obj_meshes();
        let materials_changed = self.scene.rebuild_from_world(&mut self.world);
        #[cfg(feature = "wgpu")]
        if materials_changed {
            self.invalidate_material_cache();
        }
        self.scene
            .sync_objects_to_world(&mut self.world, &self.core.object_entity_map);
        self.scene.ensure_bvh();

        // Update free flight controls (from run.rs line 86)
        if !self.paused {
            self.free_flight.update(&mut self.world, &self.input, 0.016); // Assume 60fps for app framework
        }

        // Get camera info (from run.rs line 142-160)
        let cam = self.active_camera_info();
        let cam_front = cam.orientation * Vec3::X;
        let cam_up = cam.orientation * Vec3::Y;
        let cam_right = cam.orientation * Vec3::Z;

        // Debug camera info (only print occasionally to avoid spam)
        static mut DEBUG_COUNTER: u32 = 0;
        unsafe {
            DEBUG_COUNTER += 1;
            if DEBUG_COUNTER % 300 == 0 {
                // Print every 5 seconds at 60fps
                println!(
                    "🎥 Camera: pos={:?}, front={:?}, up={:?}, right={:?}, objects={}",
                    cam.position,
                    cam_front,
                    cam_up,
                    cam_right,
                    self.scene.objects.len()
                );
                println!("   Camera orientation quat: {:?}", cam.orientation);

                // Debug object material indices
                for (i, obj) in self.scene.objects.iter().enumerate() {
                    println!(
                        "   Object {}: pos={:?}, material_idx={}, color={:?}, radius={}, is_cube={}",
                        i, obj.position, obj.material_index, obj.color, obj.radius, obj.is_cube
                    );
                }
            }
        }

        // Extract directional light from ECS (from run.rs line 161-180)
        let mut dir_light_dir = [-0.3, -0.7, -0.6]; // Default
        let mut dir_light_color = [1.0, 1.0, 0.9]; // Default
        let mut dir_light_intensity = 3.0; // Default

        // Find directional light in the world
        for entity in self.world.entities() {
            if let Some(light) = self
                .world
                .get::<crate::components::components::DirectionalLight>(*entity)
            {
                dir_light_dir = light.direction;
                dir_light_color = [light.color[0], light.color[1], light.color[2]];
                dir_light_intensity = light.intensity;
                break; // Use first directional light found
            }
        }

        // Process primitive objects from scene.objects (from run.rs line 199-288)
        // This is critical for ray tracing objects like spheres and cubes
        self.process_primitive_objects();

        // Build GPU materials and texture handles first (to avoid borrowing conflicts)
        #[cfg(feature = "wgpu")]
        let (gpu_materials, tex_handles, custom_mats, mat_names, shader_defs) =
            self.build_gpu_materials_and_textures();

        // PBR meshes will be built in the render section where view/proj matrices are available

        // Debug materials and BVH (only print occasionally to avoid spam)
        #[cfg(feature = "wgpu")]
        unsafe {
            if DEBUG_COUNTER % 300 == 0 {
                println!("   Materials: {} total", gpu_materials.len());
                for (i, mat) in gpu_materials.iter().enumerate() {
                    println!(
                        "   Material {}: base_color={:?}, roughness={}",
                        i, mat.base_color_factor, mat.roughness_factor
                    );
                }

                // Debug BVH
                println!(
                    "   BVH: {} nodes, dirty={}",
                    self.scene.bvh_nodes.len(),
                    self.scene.bvh_dirty
                );
                for (i, node) in self.scene.bvh_nodes.iter().enumerate().take(5) {
                    // Show first 5 nodes
                    println!(
                        "   BVH Node {}: min={:?}, max={:?}, children={:?}",
                        i,
                        &node.bounds_min[0..3],
                        &node.bounds_max[0..3],
                        node.children
                    );
                }
            }
        }

        // Prepare GPU data (from run.rs line 161-170)
        let (raw_gpu_objects, raw_triangles) = self.scene.get_gpu_buffers();
        let raw_atmos = self.scene.get_gpu_atmospheres();
        let raw_clouds = self.scene.get_gpu_clouds();
        let cam_pos = cam.position;
        let z_near = self.scene.camera_near_plane(cam_pos);

        // Offset all GPU objects by the camera so the camera stays at the origin
        let mut gpu_objects: Vec<_> = raw_gpu_objects.to_vec();
        for obj in &mut gpu_objects {
            obj.position[0] -= cam_pos.x;
            obj.position[1] -= cam_pos.y;
            obj.position[2] -= cam_pos.z;
        }

        // Translate triangles into world space with object transforms
        let mut gpu_triangles: Vec<_> = raw_triangles.to_vec();
        let mut tri_bvh_nodes: Vec<_> = self.scene.get_tri_bvh_nodes().to_vec();
        for obj in &self.scene.objects {
            let start = obj.triangle_start_idx;
            let end = start + obj.triangle_count;
            let b_start = obj.tri_bvh_start;
            let b_end = b_start + obj.tri_bvh_count;
            let pos = Vec3::from(obj.position);
            let rot = Quat::from_xyzw(
                obj.orientation[0],
                obj.orientation[1],
                obj.orientation[2],
                obj.orientation[3],
            );
            let rot_mat = Mat3::from_quat(rot);
            let scale = Vec3::from(obj.scale);
            for tri in &mut gpu_triangles[start..end] {
                let mut v0 = Vec3::from_array(tri.v0);
                let mut e1 = Vec3::from_array(tri.e1);
                let mut e2 = Vec3::from_array(tri.e2);
                v0 = rot_mat * (v0 * scale) + pos;
                e1 = rot_mat * (e1 * scale);
                e2 = rot_mat * (e2 * scale);
                tri.v0 = v0.to_array();
                tri.e1 = e1.to_array();
                tri.e2 = e2.to_array();
                let n0 = rot_mat * Vec3::from_array(tri.n0);
                let n1 = rot_mat * Vec3::from_array(tri.n1);
                let n2 = rot_mat * Vec3::from_array(tri.n2);
                tri.n0 = n0.normalize().to_array();
                tri.n1 = n1.normalize().to_array();
                tri.n2 = n2.normalize().to_array();
            }
            for node in &mut tri_bvh_nodes[b_start..b_end] {
                let bmin = Vec3::from_array(node.bounds_min[0..3].try_into().unwrap());
                let bmax = Vec3::from_array(node.bounds_max[0..3].try_into().unwrap());
                let corners = [
                    Vec3::new(bmin.x, bmin.y, bmin.z),
                    Vec3::new(bmin.x, bmin.y, bmax.z),
                    Vec3::new(bmin.x, bmax.y, bmin.z),
                    Vec3::new(bmin.x, bmax.y, bmax.z),
                    Vec3::new(bmax.x, bmin.y, bmin.z),
                    Vec3::new(bmax.x, bmin.y, bmax.z),
                    Vec3::new(bmax.x, bmax.y, bmin.z),
                    Vec3::new(bmax.x, bmax.y, bmax.z),
                ];
                let mut new_min = Vec3::splat(f32::INFINITY);
                let mut new_max = Vec3::splat(f32::NEG_INFINITY);
                for mut c in corners {
                    c = rot_mat * (c * scale) + pos;
                    new_min = new_min.min(c);
                    new_max = new_max.max(c);
                }
                node.bounds_min[0] = new_min.x;
                node.bounds_min[1] = new_min.y;
                node.bounds_min[2] = new_min.z;
                node.bounds_max[0] = new_max.x;
                node.bounds_max[1] = new_max.y;
                node.bounds_max[2] = new_max.z;
            }
        }
        for tri in &mut gpu_triangles {
            tri.v0[0] -= cam_pos.x;
            tri.v0[1] -= cam_pos.y;
            tri.v0[2] -= cam_pos.z;
        }

        // Shift BVH nodes so bounding volumes track the camera-relative objects
        let mut bvh_nodes: Vec<_> = self.scene.get_bvh_nodes().to_vec();
        for node in &mut bvh_nodes {
            node.bounds_min[0] -= cam_pos.x;
            node.bounds_min[1] -= cam_pos.y;
            node.bounds_min[2] -= cam_pos.z;
            node.bounds_max[0] -= cam_pos.x;
            node.bounds_max[1] -= cam_pos.y;
            node.bounds_max[2] -= cam_pos.z;
        }

        for node in &mut tri_bvh_nodes {
            node.bounds_min[0] -= cam_pos.x;
            node.bounds_min[1] -= cam_pos.y;
            node.bounds_min[2] -= cam_pos.z;
            node.bounds_max[0] -= cam_pos.x;
            node.bounds_max[1] -= cam_pos.y;
            node.bounds_max[2] -= cam_pos.z;
        }

        // Translate atmospheres relative to the camera
        let clouds: Vec<_> = raw_clouds
            .iter()
            .map(|c| {
                let mut cloud = *c;
                cloud.center_base_thickness[0] -= cam_pos.x;
                cloud.center_base_thickness[1] -= cam_pos.y;
                cloud.center_base_thickness[2] -= cam_pos.z;
                cloud
            })
            .collect();
        let atmos: Vec<_> = raw_atmos
            .iter()
            .map(|a| {
                let mut at = *a;
                at.center_radius[0] -= cam_pos.x;
                at.center_radius[1] -= cam_pos.y;
                at.center_radius[2] -= cam_pos.z;
                at
            })
            .collect();
        let have_atmos = !atmos.is_empty();

        // Get rendering settings from the active camera's PostProcessing component
        let mut gi_quality = 0u32;
        let mut gi_debug_mode = 0u32;
        let mut gi_mode = 0u32;
        let mut light_samples = 1i32;
        let mut dir_light_samples = 1i32;
        let mut max_bounces = 3i32;
        let mut raytraced_shadows_enabled = 1u32;
        let mut shadow_quality = 2u32;
        let mut max_shadow_rays = 2u32;
        let mut emissive_shadow_samples = 1u32;
        let mut directional_shadow_samples = 1u32;
        let mut cloud_object_shadows_enabled = 1u32;
        let mut dof_aperture = 0.0f32;
        let mut dof_focus_dist = 0.0f32;
        let mut dof_enable = 0u32;
        let mut atmosphere = true;
        for (ent, _cam_att) in self
            .world
            .query::<crate::components::components::CameraAttachment>()
        {
            if let Some(pp) = self
                .world
                .get::<crate::components::components::PostProcessing>(ent)
            {
                gi_quality = if pp.gi_enabled { pp.gi_quality } else { 3 };
                gi_debug_mode = pp.gi_debug_mode;
                gi_mode = if pp.path_traced_gi { 1 } else { 0 };
                light_samples = pp.light_samples as i32;
                dir_light_samples = pp.dir_light_samples as i32;
                max_bounces = pp.max_bounces as i32;
                raytraced_shadows_enabled = pp.raytraced_shadows_enabled as u32;
                shadow_quality = pp.shadow_quality.min(4);
                max_shadow_rays = pp.max_shadow_rays.min(8);
                emissive_shadow_samples = pp.emissive_shadow_samples.min(8);
                directional_shadow_samples = pp.directional_shadow_samples.min(8);
                cloud_object_shadows_enabled = pp.cloud_object_shadows_enabled as u32;
                atmosphere = pp.atmosphere;
                if let Some(d) = &pp.dof {
                    dof_enable = 1;
                    dof_aperture = d.aperture();
                    dof_focus_dist = d.focal_depth;
                }
            }
            break;
        }

        // Create render parameters (from run.rs line 343-394)
        let render_params = RenderParams {
            camera_pos: [0.0, 0.0, 0.0],
            camera_front: vec3_to_array(cam_front),
            camera_up: vec3_to_array(cam_up),
            camera_right: vec3_to_array(cam_right),
            velocity: vec3_to_array(cam.velocity),
            fov: cam.fov,
            num_objects: gpu_objects.len() as i32,
            current_time: 0.0, // Simplified - no time tracking for app framework
            skycolor: [
                self.sky_color[0] / 255.0,
                self.sky_color[1] / 255.0,
                self.sky_color[2] / 255.0,
            ],
            is_fisheye: if self.is_fisheye { 1 } else { 0 },
            selected_index: self.selection_mask,
            max_bounces,
            light_samples,
            dir_shadow_samples: dir_light_samples,
            raytraced_shadows_enabled,
            shadow_quality,
            max_shadow_rays,
            emissive_shadow_samples,
            directional_shadow_samples,
            cloud_object_shadows_enabled,
            inv_view_proj: {
                let (w, h) = self.renderer.screen_dimensions();
                let aspect = w as f32 / h as f32;
                let vp = perspective(cam.fov, aspect, z_near, 1000.0)
                    * look_at(&Vec3::ZERO, &cam_front, &cam_up);
                vp.inverse().to_cols_array_2d()
            },
            prev_view_proj: [[0.0; 4]; 4], // Simplified for app framework
            gi_quality,
            gi_debug_mode,
            gi_mode,
            dir_light_dir,
            dir_light_color: [
                dir_light_color[0] / 255.0,
                dir_light_color[1] / 255.0,
                dir_light_color[2] / 255.0,
            ],
            dir_light_intensity,
            sky_occlusion: 0.0,
            dof_aperture,
            dof_focus_dist,
            dof_enable,
            atmos,
            atmosphere: if atmosphere && have_atmos { 1 } else { 0 },
            atmosphere_mode: 0,
            cloud_history_weight: 0.88,
            cloud_sample_count: 0,
            cloud_temporal_quality: 1,
            cloud_shadow_mode: 0,
            atmosphere_sun_controls: [0.00465, 1.0, 1.0, 0.0],
            renderer_mode: crate::rendering::renderer::RendererMode::HybridEffects,
            clouds,
        };

        // Update renderer with scene data (from run.rs line 395-406)
        #[cfg(feature = "wgpu")]
        {
            self.renderer.update_scene_data(
                &gpu_objects,
                &gpu_triangles,
                &bvh_nodes,
                &tri_bvh_nodes,
                &gpu_materials,
                &custom_mats,
                &mat_names,
                &shader_defs,
                &tex_handles,
            );
        }
        #[cfg(not(feature = "wgpu"))]
        self.renderer
            .update_scene_data(&gpu_objects, &gpu_triangles, &bvh_nodes, &tri_bvh_nodes);

        // Render the frame (from run.rs line 407-502)
        #[cfg(feature = "wgpu")]
        {
            use crate::components::components::Transform;

            let (w, h) = self.renderer.screen_dimensions();
            let aspect = w as f32 / h as f32;
            let view_mat = look_at(&Vec3::ZERO, &cam_front, &cam_up);
            let proj_mat = perspective(cam.fov, aspect, z_near, 1000.0);

            // Build PBR meshes from ECS world (from run.rs line 416-437)
            use crate::gpu::MeshHandle;
            use crate::materials::PbrMaterial;
            use crate::rendering::wgpu_renderer::PbrRenderData;
            use glam::{Mat4, Quat, Vec3};

            let mut pbr_meshes = Vec::new();
            for (e, transform, mesh, mat) in
                self.world.query3::<Transform, MeshHandle, PbrMaterial>()
            {
                let model = Mat4::from_scale_rotation_translation(
                    Vec3::from(transform.size),
                    Quat::from_array([
                        transform.orientation[0],
                        transform.orientation[1],
                        transform.orientation[2],
                        transform.orientation[3],
                    ]),
                    Vec3::from(transform.position) - cam_pos,
                );
                let mvp = (proj_mat * view_mat * model).to_cols_array_2d();
                let joint_mats = if let Some(skin) = self.world.get::<Skin>(e) {
                    let mut mats = Vec::new();
                    for (joint_ent, ibm) in skin.joints.iter().zip(&skin.inverse_bind_mats) {
                        if let Some(jt) = self.world.get::<GlobalTransform>(*joint_ent) {
                            let jmat = Mat4::from_scale_rotation_translation(
                                Vec3::from(jt.size),
                                Quat::from_array([
                                    jt.orientation[0],
                                    jt.orientation[1],
                                    jt.orientation[2],
                                    jt.orientation[3],
                                ]),
                                Vec3::from(jt.position),
                            );
                            let ibm_mat = Mat4::from_cols_array_2d(ibm);
                            let final_mat = jmat * ibm_mat;
                            mats.push(final_mat.to_cols_array_2d());
                        }
                    }
                    Some(mats)
                } else {
                    None
                };
                pbr_meshes.push(PbrRenderData {
                    mesh: mesh.clone(),
                    material: mat.clone(),
                    mvp,
                    model: model.to_cols_array_2d(),
                    joint_mats,
                });
            }

            // Handle EGUI rendering for wgpu
            #[cfg(feature = "use_epi")]
            {
                // Create EGUI frame (similar to run.rs line 100-125)
                let (logical_w, logical_h) = self.window.window.size();
                let (drawable_w, drawable_h) = self.window.window.drawable_size();
                let screen_size = egui::vec2(logical_w as f32, logical_h as f32);
                let pixels_per_point = drawable_w as f32 / logical_w.max(1) as f32;

                self.egui_ctx.set_pixels_per_point(pixels_per_point);
                let mut egui_input = egui::RawInput::default();
                egui_input.screen_rect =
                    Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size));
                egui_input.events = std::mem::take(&mut self.egui_events);
                egui_input.predicted_dt = 1.0 / 60.0;
                egui_input.focused = true;
                egui_input.viewports.insert(
                    egui::ViewportId::ROOT,
                    egui::ViewportInfo {
                        inner_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size)),
                        native_pixels_per_point: Some(pixels_per_point),
                        ..Default::default()
                    },
                );

                // Run EGUI frame and call draw_editor_ui (using unsafe pointer like run.rs)
                let engine_ptr = self as *mut Engine;
                let full_output = self.egui_ctx.run(egui_input, |ctx| {
                    let engine: &mut Engine = unsafe { &mut *engine_ptr };
                    engine.draw_editor_ui(ctx);
                });

                let shapes = full_output.shapes;
                let textures_delta = full_output.textures_delta;
                let paint_jobs = self
                    .egui_ctx
                    .tessellate(shapes, self.egui_ctx.pixels_per_point());

                // Render with wgpu including EGUI
                self.renderer.render(
                    &render_params,
                    &[],
                    &pbr_meshes,
                    Some((&mut self.egui_renderer, &paint_jobs, &textures_delta)),
                );
            }
            #[cfg(not(feature = "use_epi"))]
            {
                // Render with wgpu without EGUI
                self.renderer.render(&render_params, &[], &pbr_meshes, None);
            }
        }
        #[cfg(not(feature = "wgpu"))]
        {
            // Render with OpenGL
            self.renderer.render(&render_params);
        }

        // Handle EGUI rendering for OpenGL (from run.rs line 519-531)
        #[cfg(all(not(feature = "wgpu"), feature = "use_epi"))]
        {
            let paint_jobs = self
                .egui_ctx
                .tessellate(shapes, self.egui_ctx.pixels_per_point());
            self.egui_renderer
                .paint_jobs(None, textures_delta, paint_jobs);
        }

        // Post-render operations (from run.rs line 503-538)
        #[cfg(not(feature = "wgpu"))]
        {
            self.renderer.capture_screen();
            // Note: Blur regions functionality moved to editor plugin
            let regions: Vec<(i32, i32, i32, i32)> = Vec::new();
            self.renderer.blur_regions(&regions, 10.0);
        }

        // CRITICAL: Swap buffers to display the rendered frame (from run.rs line 527-538)
        #[cfg(not(feature = "wgpu"))]
        {
            self.window.swap_buffers();
        }
        // For wgpu, the buffer swapping is handled internally by the renderer

        // Swap buffers and check for window close (from run.rs line 519-544)
        #[cfg(all(not(feature = "wgpu"), feature = "use_epi"))]
        {
            self.window.swap_buffers();
            if self.window.should_close() {
                self.running = false;
            }
        }
        #[cfg(all(not(feature = "wgpu"), not(feature = "use_epi")))]
        {
            self.window.swap_buffers();
            if self.window.should_close() {
                self.running = false;
            }
        }
        #[cfg(feature = "wgpu")]
        {
            if self.window.should_close() {
                self.running = false;
            }
        }
    }

    /// Change the window resolution and resize renderer accordingly.
    pub fn set_window_size(&mut self, width: u32, height: u32) {
        self.window.resize(width as i32, height as i32);
        self.renderer.resize(width as i32, height as i32);
        #[cfg(feature = "use_epi")]
        self.egui_renderer.update_screen_rect((width, height));
    }

    /// Adjust internal rendering resolution scale (0.1-1.0).
    pub fn set_render_scale(&mut self, scale: f32) {
        self.renderer.set_render_scale(scale);
    }

    /// Enable AMD FSR upscaling with a given sharpness factor.
    pub fn enable_fsr(&mut self, sharpness: f32) {
        self.renderer.enable_fsr(sharpness);
    }

    /// Disable AMD FSR upscaling.
    pub fn disable_fsr(&mut self) {
        self.renderer.disable_fsr();
    }

    /// Render EGUI UI with a callback function
    /// This allows plugins to render their UI during the EGUI frame
    pub fn render_ui<F>(&mut self, ui_callback: F)
    where
        F: FnOnce(&egui::Context),
    {
        #[cfg(feature = "use_epi")]
        {
            // Setup EGUI frame
            let (w, h) = self.renderer.screen_dimensions();
            let screen_size = egui::vec2(w as f32, h as f32);
            let pixels_per_point = 1.0;

            self.egui_ctx.set_pixels_per_point(pixels_per_point);
            let mut egui_input = egui::RawInput::default();
            egui_input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size));
            egui_input.events = std::mem::take(&mut self.egui_events);
            egui_input.predicted_dt = 1.0 / 60.0;
            egui_input.focused = true;
            egui_input.viewports.insert(
                egui::ViewportId::ROOT,
                egui::ViewportInfo {
                    inner_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size)),
                    native_pixels_per_point: Some(pixels_per_point),
                    ..Default::default()
                },
            );

            // Run EGUI frame with callback
            let full_output = self.egui_ctx.run(egui_input, |ctx| {
                ui_callback(ctx);
            });

            let shapes = full_output.shapes;
            let textures_delta = full_output.textures_delta;

            // Render EGUI
            #[cfg(not(feature = "wgpu"))]
            {
                let paint_jobs = self
                    .egui_ctx
                    .tessellate(shapes, self.egui_ctx.pixels_per_point());
                self.egui_renderer
                    .paint_jobs(None, textures_delta, paint_jobs);
            }
        }
    }

    /// Process primitive objects from scene.objects (spheres, cubes, etc.)
    /// This replicates the primitive object processing from run.rs line 199-288
    fn process_primitive_objects(&mut self) {
        use crate::CustomMaterial;
        use crate::scene::object::GpuMaterial;
        use std::collections::HashMap;

        // Assemble GPU materials for every scene object, generating
        // defaults for primitives that lack an explicit `PbrMaterial`
        let mut gpu_materials: Vec<GpuMaterial> = Vec::new();
        let mut mat_map: HashMap<String, u32> = HashMap::new();

        // Preload materials referenced by triangles so indices match
        for mat in &self.scene.materials {
            let idx = gpu_materials.len() as u32;
            mat_map.insert(mat.name.clone(), idx);
            let mut f0 = mat.specular_f0;
            if f0 == [0.0; 3] {
                let f = (mat.ior - 1.0) / (mat.ior + 1.0);
                f0 = [f * f; 3];
            }
            let emissive_strength = mat.emissive.iter().fold(0.0_f32, |a, &b| a.max(b));
            let emissive_factor = if emissive_strength > 0.0 {
                [
                    mat.emissive[0] / emissive_strength,
                    mat.emissive[1] / emissive_strength,
                    mat.emissive[2] / emissive_strength,
                ]
            } else {
                [0.0; 3]
            };
            gpu_materials.push(GpuMaterial {
                base_color_factor: mat.base_color,
                emissive_factor,
                emissive_strength,
                metallic_factor: mat.metallic,
                roughness_factor: mat.roughness,
                ior: mat.ior,
                base_color_tex: 0, // No texture support for primitives yet
                f0,
                ..Default::default()
            });
        }

        // Process each primitive object in the scene
        for (i, obj) in self.scene.objects.iter_mut().enumerate() {
            // See if the object has a material component in the world
            let entity_mat = self
                .core
                .object_entity_map
                .get(&(i as u32))
                .and_then(|&entity| self.world.get::<crate::materials::PbrMaterial>(entity));

            let idx = if let Some(mat) = entity_mat {
                // Use material from ECS component
                let mat_name = format!("entity_material_{}", i);
                *mat_map.entry(mat_name.clone()).or_insert_with(|| {
                    let idx = gpu_materials.len() as u32;
                    gpu_materials.push(GpuMaterial {
                        base_color_factor: mat.base_color,
                        emissive_factor: [0.0; 3], // Simplified
                        emissive_strength: 0.0,
                        metallic_factor: mat.metallic,
                        roughness_factor: mat.roughness,
                        ior: 1.5, // Default IOR
                        base_color_tex: 0,
                        f0: [0.04; 3], // Default F0 for dielectrics
                        ..Default::default()
                    });
                    idx
                })
            } else {
                // Create default material for primitive
                let mat_name = format!("default_primitive_{}", i);
                *mat_map.entry(mat_name.clone()).or_insert_with(|| {
                    let idx = gpu_materials.len() as u32;
                    gpu_materials.push(GpuMaterial {
                        base_color_factor: [
                            obj.color[0] / 255.0,
                            obj.color[1] / 255.0,
                            obj.color[2] / 255.0,
                            1.0,
                        ],
                        emissive_factor: [0.0; 3],
                        emissive_strength: 0.0,
                        metallic_factor: 0.0,
                        roughness_factor: 0.5,
                        ior: 1.5,
                        base_color_tex: 0,
                        f0: [0.04; 3],
                        ..Default::default()
                    });
                    idx
                })
            };
            if let Some(entity) = self.core.object_entity_map.get(&(i as u32)) {
                if self.world.get::<CustomMaterial>(*entity).is_some() {
                    if let Some(m) = gpu_materials.get_mut(idx as usize) {
                        m.has_custom_material = 1;
                    }
                }
            }
            obj.material_index = idx;
        }

        // Rebuild GPU objects with updated material indices
        self.scene.gpu_objects = self.scene.objects.iter().map(|o| o.to_gpu()).collect();
    }

    /// Register a named script event for an entity if it doesn't exist.
    pub fn define_signal(&mut self, entity: Entity, name: &str) {
        self.scene_events
            .script_events
            .entry((entity, name.to_string()))
            .or_insert_with(LuaEvent::new);
    }

    /// Emit a script event for `entity` with the given value.
    pub fn emit_signal(&mut self, entity: Entity, name: &str, val: LuaValue) {
        if let Some(ev) = self
            .scene_events
            .script_events
            .get_mut(&(entity, name.to_string()))
        {
            ev.emit(val);
        }
    }

    pub fn emit_signal_string(&mut self, entity: Entity, name: &str, text: &str) {
        if let Some(ev) = self
            .scene_events
            .script_events
            .get_mut(&(entity, name.to_string()))
        {
            ev.emit_string(text);
        }
    }

    /// Subscribe a callback to a script event on `entity`.
    pub fn subscribe_signal(&mut self, entity: Entity, name: &str, func: Function) {
        self.scene_events
            .script_events
            .entry((entity, name.to_string()))
            .or_insert_with(LuaEvent::new)
            .subscribe(func);
    }

    /// Ensure a global event exists.
    pub fn define_event(&mut self, name: &str) {
        self.scene_events
            .global_events
            .entry(name.to_string())
            .or_default();
    }

    /// Emit a global event to all listeners.
    pub fn emit_event(&mut self, name: &str, sender: Entity, val: LuaValue) {
        if let Some(list) = self.scene_events.global_events.get_mut(name) {
            let callbacks: Vec<*mut dyn FnMut(&mut Engine, Entity, LuaValue)> =
                list.iter_mut().map(|f| &mut **f as *mut _).collect();
            for cb in callbacks {
                unsafe {
                    (*cb)(self, sender, val.clone());
                }
            }
        }
    }

    /// Subscribe to a global event with a Lua callback.
    pub fn subscribe_event(&mut self, lua: &Lua, name: &str, func: Function) {
        let lua = lua.clone();
        let f = func.clone();
        self.scene_events
            .global_events
            .entry(name.to_string())
            .or_default()
            .push(Box::new(move |engine, sender, value| {
                if let Ok(ud) = lua.create_userdata(EntityProxy::new(engine as *mut Engine, sender))
                {
                    let _ = f.call::<()>((ud, value.clone()));
                }
            }));
    }
    /// Export the current scene to a [`SceneFile`] for later restoration.
    pub fn export_scene(&mut self) -> SceneFile {
        let mut nodes = Vec::new();
        let mut entities = Vec::new();
        for idx in 0..self.scene.objects.len() {
            let obj = self.scene.objects[idx];
            let mut components = Vec::new();
            if let Some(entity) = self.core.find_entity_by_object_id(idx as u32) {
                let comp_names = self.list_components_entity(entity);
                for name in comp_names {
                    if name == "Metadata" || name == "ObjectRef" {
                        continue;
                    }
                    if let Some(comp) = self.access_component_mut(entity, &name) {
                        let mut map = Map::new();
                        for field in comp.exported_fields_mut() {
                            unsafe {
                                let val = if field.type_id == std::any::TypeId::of::<f32>() {
                                    Value::from(*(field.value as *mut f32))
                                } else if field.type_id == std::any::TypeId::of::<f64>() {
                                    Value::from(*(field.value as *mut f64))
                                } else if field.type_id == std::any::TypeId::of::<i32>() {
                                    Value::from(*(field.value as *mut i32))
                                } else if field.type_id == std::any::TypeId::of::<u32>() {
                                    Value::from(*(field.value as *mut u32))
                                } else if field.type_id == std::any::TypeId::of::<bool>() {
                                    Value::from(*(field.value as *mut bool))
                                } else if field.type_id == std::any::TypeId::of::<String>() {
                                    Value::from((*(field.value as *mut String)).clone())
                                } else {
                                    continue;
                                };
                                map.insert(field.name.to_string(), val);
                            }
                        }
                        components.push(ComponentFile {
                            name,
                            data: Value::Object(map),
                        });
                    }
                }
            }
            let meta = self
                .core
                .find_entity_by_object_id(idx as u32)
                .and_then(|e| self.world.get::<crate::components::components::Metadata>(e));
            nodes.push(NodeFile {
                name: meta
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| format!("Object{}", idx)),
                tags: meta.map(|m| m.tags.clone()).unwrap_or_default(),
                position: obj.position,
                color: obj.color,
                size: obj.size,
                scale: obj.scale,
                is_cube: obj.is_cube,
                components,
            });
        }
        let entity_list: Vec<_> = self.world.entities().iter().copied().collect();
        for entity in entity_list {
            if self
                .world
                .get::<crate::components::components::ObjectRef>(entity)
                .is_some()
            {
                continue;
            }
            let (name, tags) = self
                .world
                .get::<crate::components::components::Metadata>(entity)
                .map(|m| (m.name.clone(), m.tags.clone()))
                .unwrap_or_else(|| (format!("Entity{}", entity.0), Vec::new()));
            let mut comps = Vec::new();
            for cname in self.list_components_entity(entity) {
                if cname == "Metadata" || cname == "ObjectRef" {
                    continue;
                }
                if let Some(comp) = self.access_component_mut(entity, &cname) {
                    comps.push(ComponentFile {
                        name: cname,
                        data: export_component_data(comp),
                    });
                }
            }
            entities.push(EntityFile {
                name,
                tags,
                components: comps,
            });
        }

        SceneFile { nodes, entities }
    }

    /// Pause all running systems and store the scene for restart.
    pub fn pause(&mut self) {
        if !self.paused {
            if self.saved_scene.is_none() {
                self.saved_scene = Some(self.export_scene());
            }
            for (_e, src) in self
                .world
                .query_mut::<crate::components::components::AudioSource>()
            {
                if src.state == crate::components::components::AudioPlayState::Playing {
                    src.state = crate::components::components::AudioPlayState::Paused;
                }
            }
            self.paused = true;
        }
    }

    /// Resume simulation from a paused state.
    pub fn resume(&mut self) {
        if self.paused {
            self.saved_scene = Some(self.export_scene());
            for (_e, src) in self
                .world
                .query_mut::<crate::components::components::AudioSource>()
            {
                if src.state == crate::components::components::AudioPlayState::Paused {
                    src.state = crate::components::components::AudioPlayState::Playing;
                }
            }
            self.paused = false;
        }
    }

    /// Reload the scene from the stored snapshot and continue running.
    pub fn restart(&mut self) {
        if let Some(scene) = self.saved_scene.clone() {
            self.clear_scene();
            let _ = self.load_scene(scene);
        }
        self.paused = false;
    }

    /// Build GPU materials and texture handles like the legacy engine does
    /// This is extracted from run.rs lines 142-340
    #[cfg(feature = "wgpu")]
    fn build_gpu_materials_and_textures(
        &mut self,
    ) -> (
        Vec<crate::scene::object::GpuMaterial>,
        Vec<crate::gpu::TextureHandle>,
        Vec<crate::scene::object::GpuCustomMaterial>,
        Vec<String>,
        Vec<(String, String)>,
    ) {
        if !self.materials_dirty && !self.cached_gpu_materials.is_empty() {
            return (
                self.cached_gpu_materials.clone(),
                self.cached_tex_handles.clone(),
                self.cached_custom_materials.clone(),
                self.cached_custom_names.clone(),
                self.cached_shader_defs.clone(),
            );
        }

        use crate::gpu::TextureHandle;
        use crate::materials::PbrMaterial;
        use crate::scene::object::GpuMaterial;
        use std::collections::HashMap;

        // Assemble GPU materials for every scene object, generating
        // defaults for primitives that lack an explicit `PbrMaterial`
        let mut gpu_materials: Vec<GpuMaterial> = Vec::new();
        let mut custom_materials: Vec<crate::scene::object::GpuCustomMaterial> = Vec::new();
        let mut material_names: Vec<String> = Vec::new();
        let mut shader_defs: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut mat_map: HashMap<String, u32> = HashMap::new();
        let mut tex_map: HashMap<*const crate::gpu::GpuTexture, u32> = HashMap::new();
        let mut tex_handles: Vec<TextureHandle> = Vec::new();

        // Index 0 reserved for white texture fallback
        let white = self.renderer.white_texture_handle();
        tex_map.insert(std::sync::Arc::as_ptr(&white.0), 0);
        tex_handles.push(white.clone());

        // Preload materials referenced by triangles so indices match
        for mat in &self.scene.materials {
            let idx = gpu_materials.len() as u32;
            mat_map.insert(mat.name.clone(), idx);
            let mut f0 = mat.specular_f0;
            if f0 == [0.0; 3] {
                let f = (mat.ior - 1.0) / (mat.ior + 1.0);
                f0 = [f * f; 3];
            }
            let emissive_strength = mat.emissive.iter().fold(0.0_f32, |a, &b| a.max(b));
            let emissive_factor = if emissive_strength > 0.0 {
                [
                    mat.emissive[0] / emissive_strength,
                    mat.emissive[1] / emissive_strength,
                    mat.emissive[2] / emissive_strength,
                ]
            } else {
                [0.0; 3]
            };
            let tex_idx = if let Some(tex) = mat.base_color_tex.clone() {
                let ptr = std::sync::Arc::as_ptr(&tex.0);
                *tex_map.entry(ptr).or_insert_with(|| {
                    let idx = tex_handles.len() as u32 + 1;
                    tex_handles.push(tex.clone());
                    idx
                })
            } else {
                0
            };
            gpu_materials.push(GpuMaterial {
                base_color_factor: mat.base_color,
                emissive_factor,
                emissive_strength,
                metallic_factor: mat.metallic,
                roughness_factor: mat.roughness,
                ior: mat.ior,
                base_color_tex: tex_idx,
                f0,
                ..Default::default()
            });
        }

        // Process scene objects and create materials for them
        let time = self.renderer.frame_number() as f32 * (1.0 / 60.0);
        for (i, obj) in self.scene.objects.iter_mut().enumerate() {
            // See if the object has a material component in the world
            let entity_mat = self
                .core
                .object_entity_map
                .get(&(i as u32))
                .and_then(|e| self.world.get::<PbrMaterial>(*e));

            let idx = if let Some(mat) = entity_mat {
                *mat_map.entry(mat.name.clone()).or_insert_with(|| {
                    let idx = gpu_materials.len() as u32;
                    let mut f0 = mat.specular_f0;
                    if f0 == [0.0; 3] {
                        let f = (mat.ior - 1.0) / (mat.ior + 1.0);
                        f0 = [f * f; 3];
                    }
                    let emissive_strength = mat.emissive.iter().fold(0.0_f32, |a, &b| a.max(b));
                    let emissive_factor = if emissive_strength > 0.0 {
                        [
                            mat.emissive[0] / emissive_strength,
                            mat.emissive[1] / emissive_strength,
                            mat.emissive[2] / emissive_strength,
                        ]
                    } else {
                        [0.0; 3]
                    };
                    let tex_idx = if let Some(tex) = mat.base_color_tex.clone() {
                        let ptr = std::sync::Arc::as_ptr(&tex.0);
                        *tex_map.entry(ptr).or_insert_with(|| {
                            let idx = tex_handles.len() as u32 + 1;
                            tex_handles.push(tex.clone());
                            idx
                        })
                    } else {
                        0
                    };
                    gpu_materials.push(GpuMaterial {
                        base_color_factor: mat.base_color,
                        emissive_factor,
                        emissive_strength,
                        metallic_factor: mat.metallic,
                        roughness_factor: mat.roughness,
                        ior: mat.ior,
                        base_color_tex: tex_idx,
                        f0,
                        ..Default::default()
                    });
                    idx
                })
            } else {
                let idx = gpu_materials.len() as u32;
                let mut f0 = obj.specular_f0;
                if f0 == [0.0; 3] {
                    let f = (obj.ior - 1.0) / (obj.ior + 1.0);
                    f0 = [f * f; 3];
                }
                // Object colors are already stored in linear 0-1 range, so use them directly
                let base_color_factor = [obj.color[0], obj.color[1], obj.color[2], 1.0];
                let emissive_strength = obj.emission;
                let emissive_factor = if emissive_strength > 0.0 {
                    [
                        base_color_factor[0],
                        base_color_factor[1],
                        base_color_factor[2],
                    ]
                } else {
                    [0.0; 3]
                };
                gpu_materials.push(GpuMaterial {
                    base_color_factor,
                    emissive_factor,
                    emissive_strength,
                    metallic_factor: 0.0, // Objects don't have metallic property, default to 0
                    roughness_factor: obj.roughness,
                    ior: obj.ior,
                    base_color_tex: 0,
                    f0,
                    ..Default::default()
                });
                idx
            };

            // Update the object's material index
            obj.material_index = idx;
            if let Some(entity) = self.core.object_entity_map.get(&(i as u32)) {
                if let Some(custom) = self.world.get::<CustomMaterial>(*entity) {
                    let id = custom_materials.len() as u32;
                    if let Some(m) = gpu_materials.get_mut(idx as usize) {
                        m.has_custom_material = 1;
                        m.custom_material_id = id;
                    }
                    let mut gpu = crate::scene::object::GpuCustomMaterial::default();
                    for (k, v) in &custom.parameters {
                        match (k.as_str(), v) {
                            ("color_tint", MaterialParameter::Vec3(v)) => {
                                gpu.color_tint = [v[0], v[1], v[2], 1.0]
                            }
                            ("roughness", MaterialParameter::Float(f)) => gpu.base_props[0] = *f,
                            ("metallic", MaterialParameter::Float(f)) => gpu.base_props[1] = *f,
                            ("noise_scale", MaterialParameter::Float(f)) => gpu.base_props[2] = *f,
                            ("emission_strength", MaterialParameter::Float(f)) => {
                                gpu.base_props[3] = *f
                            }
                            ("custom_float_1", MaterialParameter::Float(f))
                            | ("rainbow_scale", MaterialParameter::Float(f)) => {
                                gpu.custom_floats[0] = *f
                            }
                            ("custom_float_2", MaterialParameter::Float(f))
                            | ("speed", MaterialParameter::Float(f)) => gpu.custom_floats[1] = *f,
                            ("custom_float_3", MaterialParameter::Float(f))
                            | ("glow_strength", MaterialParameter::Float(f)) => {
                                gpu.custom_floats[2] = *f
                            }
                            ("custom_float_4", MaterialParameter::Float(f)) => {
                                gpu.custom_floats[3] = *f
                            }
                            ("transparency", MaterialParameter::Float(f)) => {
                                gpu.transparency_params[0] = *f
                            }
                            ("transmission", MaterialParameter::Float(f)) => {
                                gpu.transparency_params[1] = *f
                            }
                            ("transmission_roughness", MaterialParameter::Float(f)) => {
                                gpu.transparency_params[2] = *f
                            }
                            ("refraction_ior", MaterialParameter::Float(f)) => {
                                gpu.transparency_params[3] = *f
                            }
                            ("subsurface_strength", MaterialParameter::Float(f)) => {
                                gpu.subsurface_params[0] = *f
                            }
                            ("subsurface_radius", MaterialParameter::Vec3(v)) => {
                                gpu.subsurface_params[1] = v[0];
                                gpu.subsurface_params[2] = v[1];
                                gpu.subsurface_params[3] = v[2];
                            }
                            ("clearcoat_strength", MaterialParameter::Float(f)) => {
                                gpu.coat_aniso[0] = *f
                            }
                            ("clearcoat_roughness", MaterialParameter::Float(f)) => {
                                gpu.coat_aniso[1] = *f
                            }
                            ("anisotropy", MaterialParameter::Float(f)) => gpu.coat_aniso[2] = *f,
                            ("anisotropy_rotation", MaterialParameter::Float(f)) => {
                                gpu.coat_aniso[3] = *f
                            }
                            ("sheen_strength", MaterialParameter::Float(f)) => {
                                gpu.sheen_params[0] = *f
                            }
                            ("sheen_tint", MaterialParameter::Vec3(v)) => {
                                gpu.sheen_params[1] = v[0];
                                gpu.sheen_params[2] = v[1];
                                gpu.sheen_params[3] = v[2];
                            }
                            ("normal_strength", MaterialParameter::Float(f)) => {
                                gpu.normal_disp[0] = *f
                            }
                            ("displacement_strength", MaterialParameter::Float(f)) => {
                                gpu.normal_disp[1] = *f
                            }
                            ("texture", MaterialParameter::Texture(tex)) => {
                                let ptr = std::sync::Arc::as_ptr(&tex.0);
                                let tex_idx = *tex_map.entry(ptr).or_insert_with(|| {
                                    let idx = tex_handles.len() as u32 + 1;
                                    tex_handles.push(tex.clone());
                                    idx
                                });
                                gpu.texture_index = tex_idx;
                            }
                            _ => {}
                        }
                    }
                    gpu.custom_floats[3] = time;
                    custom_materials.push(gpu);
                    material_names.push(custom.material_type.clone());
                    shader_defs
                        .entry(custom.material_type.clone())
                        .or_insert(custom.shader_source.clone());
                }
            }
        }

        self.cached_gpu_materials = gpu_materials.clone();
        self.cached_tex_handles = tex_handles.clone();
        self.cached_custom_materials = custom_materials.clone();
        self.cached_custom_names = material_names.clone();
        let shader_defs_vec: Vec<(String, String)> = shader_defs.into_iter().collect();
        self.cached_shader_defs = shader_defs_vec.clone();
        self.materials_dirty = false;
        (
            gpu_materials,
            tex_handles,
            custom_materials,
            material_names,
            shader_defs_vec,
        )
    }
}
