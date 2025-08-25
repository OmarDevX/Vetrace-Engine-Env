use super::engine::{sdl_event_to_egui_event, EmptyBehaviour};
use super::Engine;
use crate::components::components::ObjectRef;
use crate::components::components::Sprite3D;
use crate::gpu::{MeshHandle, TextureHandle};
use crate::materials::PbrMaterial;
use crate::CustomMaterial;
use crate::math::{look_at, perspective, vec3_to_array};
#[cfg(feature = "wgpu")]
use crate::rendering::wgpu_renderer::{PbrRenderData, SpriteRenderData};
use crate::rendering::RenderParams;
use crate::scene::object::GpuMaterial;
#[cfg(not(feature = "wgpu"))]
use crate::systems::sprite_render::SpriteRenderSystem;
use crate::Behaviour;
use egui::{Pos2, Rect, ViewportId, ViewportInfo};
use glam::{Mat3, Mat4, Quat, Vec3};
use sdl2::event::Event as SdlEvent;
use sdl2::mouse::MouseState;
use std::collections::HashMap;
use std::time::Instant;

impl Engine {
    pub fn run_with_behaviour<B: Behaviour + 'static>(
        &mut self,
        enable_editor: bool,
        mut behaviour: B,
    ) {
        behaviour.start(self);
        let mut behaviours = std::mem::take(&mut self.behaviours);
        for b in behaviours.iter_mut() {
            b.start(self);
        }
        self.behaviours = behaviours;
        self.start_script_components();
        self.start_component_behaviours();
        if self.saved_scene.is_none() {
            self.saved_scene = Some(self.export_scene());
        }

        let start_time = Instant::now();
        let mut last_frame_time = Instant::now();

        self.window.video_subsystem.text_input().start();
        while self.running {
            let now = Instant::now();
            let delta = if self.paused {
                last_frame_time = now;
                0.0
            } else {
                let d = (now - last_frame_time).as_secs_f32();
                last_frame_time = now;
                d
            };

            self.input.begin_frame();
            self.egui_events.clear();

            let mouse_state = sdl2::mouse::MouseState::new(&self.window.event_pump);
            let mouse_pos = Pos2::new(mouse_state.x() as f32, mouse_state.y() as f32);

            let events: Vec<_> = self.window.poll_iter().collect();
            for event in events {
                self.input.update(&event);
                if let Some(e) = sdl_event_to_egui_event(&event, mouse_pos) {
                    self.egui_events.push(e);
                }
                match event {
                    SdlEvent::Quit { .. } => {
                        self.running = false;
                    }
                    SdlEvent::Window { win_event, .. } => match win_event {
                        sdl2::event::WindowEvent::Resized(w, h)
                        | sdl2::event::WindowEvent::SizeChanged(w, h) => {
                            self.window.resize(w, h);
                            self.renderer.resize(w, h);
                            #[cfg(feature = "use_epi")]
                            self.egui_renderer.update_screen_rect((w as u32, h as u32));
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if !self.paused {
                self.free_flight.update(&mut self.world, &self.input, delta);

                behaviour.update(self, delta);
                self.update_script_components(delta);
                self.update_component_behaviours(delta);
                let mut behaviours = std::mem::take(&mut self.behaviours);
                for b in behaviours.iter_mut() {
                    b.update(self, delta);
                }
                self.behaviours = behaviours;
            }

            // Keep the camera at the origin by shifting the world
            let cam_pos = self.active_camera_info().position;
            if cam_pos.length_squared() > 0.0 {
                self.shift_origin(cam_pos);
            }

            let (logical_w, logical_h) = self.window.get_size();
            let (drawable_w, _) = self.window.window.drawable_size();
            let screen_size = egui::vec2(logical_w as f32, logical_h as f32);
            let pixels_per_point = drawable_w as f32 / logical_w.max(1) as f32;

            let engine_ptr = self as *mut Engine;
            self.egui_ctx.set_pixels_per_point(pixels_per_point);
            let mut egui_input = egui::RawInput::default();
            egui_input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, screen_size));
            egui_input.events = std::mem::take(&mut self.egui_events);
            egui_input.predicted_dt = 1.0 / 60.0;
            egui_input.focused = true;
            egui_input.viewports.insert(
                ViewportId::ROOT,
                ViewportInfo {
                    inner_rect: Some(Rect::from_min_size(Pos2::ZERO, screen_size)),
                    native_pixels_per_point: Some(pixels_per_point),
                    ..Default::default()
                },
            );

            let full_output = self.egui_ctx.run(egui_input, |ctx| {
                let engine: &mut Engine = unsafe { &mut *engine_ptr };
                engine.draw_game_ui(ctx);
                if enable_editor {
                    engine.draw_editor_ui(ctx);
                }
            });
            let shapes = full_output.shapes;
            let textures_delta = full_output.textures_delta;
            #[cfg(feature = "wgpu")]
            {
                // Note: Blur regions functionality moved to editor plugin
                let regions: Vec<(i32, i32, i32, i32)> = Vec::new();
                self.renderer.blur_regions(&regions, 10.0);
            }

            self.update_obj_meshes();
            let materials_changed = self.scene.rebuild_from_world(&mut self.world);
            #[cfg(feature = "wgpu")]
            if materials_changed {
                self.invalidate_material_cache();
            }
            // Object simulation is now fully handled by Rapier, so we only
            // rebuild GPU data from the ECS world.
            self.scene
                .sync_objects_to_world(&mut self.world, &self.core.object_entity_map);
            self.scene.ensure_bvh();

            // Assemble GPU materials for every scene object, generating
            // defaults for primitives that lack an explicit `PbrMaterial`
            let mut gpu_materials: Vec<GpuMaterial> = Vec::new();
            let mut mat_map: HashMap<String, u32> = HashMap::new();
            let mut tex_map: HashMap<*const crate::gpu::GpuTexture, u32> = HashMap::new();
            let mut tex_handles: Vec<TextureHandle> = Vec::new();
            // Index 0 reserved for white texture fallback
            #[cfg(feature = "wgpu")]
            {
                let white = self.renderer.white_texture_handle();
                tex_map.insert(std::sync::Arc::as_ptr(&white.0), 0);
                tex_handles.push(white.clone());
            }

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

            let cam = self.active_camera_info();
            let scene = &mut self.scene;
            for (i, obj) in scene.objects.iter_mut().enumerate() {
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
                    let base_color_factor = [
                        obj.color[0],
                        obj.color[1],
                        obj.color[2],
                        1.0,
                    ];
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
                        metallic_factor: 0.0,
                        roughness_factor: obj.roughness,
                        ior: obj.ior,
                        base_color_tex: 0,
                        f0,
                        ..Default::default()
                    });
                    idx
                };
                if let Some(entity) = self.core.object_entity_map.get(&(i as u32)) {
                    if self.world.get::<CustomMaterial>(*entity).is_some() {
                        if let Some(m) = gpu_materials.get_mut(idx as usize) {
                            m.has_custom_material = 1;
                        }
                    }
                }
                obj.material_index = idx;
                let start = obj.triangle_start_idx;
                let end = start + obj.triangle_count;
                for tri in &mut scene.triangles[start..end] {
                    tri.material_index = idx;
                }
            }

            // Rebuild GPU objects with updated material indices
            scene.gpu_objects = scene.objects.iter().map(|o| o.to_gpu()).collect();

            let (raw_gpu_objects, raw_triangles) = scene.get_gpu_buffers();
            let raw_atmos = scene.get_gpu_atmospheres();
            let cam_pos = cam.position;
            let cam_front = cam.orientation * Vec3::X;
            let cam_up = cam.orientation * Vec3::Y;
            let cam_right = cam.orientation * Vec3::Z;

            let mut gpu_objects: Vec<_> = raw_gpu_objects.to_vec();
            for obj in &mut gpu_objects {
                obj.position[0] -= cam_pos.x;
                obj.position[1] -= cam_pos.y;
                obj.position[2] -= cam_pos.z;
            }
            let mut gpu_triangles: Vec<_> = raw_triangles.to_vec();
            let mut tri_bvh_nodes: Vec<_> = scene.get_tri_bvh_nodes().to_vec();
            for obj in &scene.objects {
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
            let mut bvh_nodes: Vec<_> = scene.get_bvh_nodes().to_vec();
            for node in &mut bvh_nodes {
                node.center_radius[0] -= cam_pos.x;
                node.center_radius[1] -= cam_pos.y;
                node.center_radius[2] -= cam_pos.z;
            }
            for node in &mut tri_bvh_nodes {
                node.bounds_min[0] -= cam_pos.x;
                node.bounds_min[1] -= cam_pos.y;
                node.bounds_min[2] -= cam_pos.z;
                node.bounds_max[0] -= cam_pos.x;
                node.bounds_max[1] -= cam_pos.y;
                node.bounds_max[2] -= cam_pos.z;
            }

            let mut gi_quality = 0u32;
            let mut gi_debug_mode = 0u32;
            let mut gi_mode = 0u32;
            let mut light_samples = 1i32;
            let mut dir_light_samples = 1i32;
            let mut max_bounces = 3i32;
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
                    atmosphere = pp.atmosphere;
                    if let Some(d) = &pp.dof {
                        dof_enable = 1;
                        dof_aperture = d.aperture();
                        dof_focus_dist = d.focal_depth;
                    }
                }
                break;
            }

            let mut dir_light = crate::components::components::DirectionalLight::default();
            for (_e, light) in self
                .world
                .query::<crate::components::components::DirectionalLight>()
            {
                dir_light = *light;
                break;
            }

            let render_params = RenderParams {
                camera_pos: [0.0, 0.0, 0.0],
                camera_front: vec3_to_array(cam_front),
                camera_up: vec3_to_array(cam_up),
                camera_right: vec3_to_array(cam_right),
                velocity: vec3_to_array(cam.velocity),
                fov: cam.fov,
                num_objects: gpu_objects.len() as i32,
                current_time: (now - start_time).as_secs_f32(),
                skycolor: [
                    self.sky_color[0] / 255.0,
                    self.sky_color[1] / 255.0,
                    self.sky_color[2] / 255.0,
                ],
                is_fisheye: if self.is_fisheye { 1 } else { 0 },
                selected_index: 0, // No selection (moved to editor plugin)
                max_bounces,
                light_samples,
                dir_shadow_samples: dir_light_samples,
                inv_view_proj: {
                    let (w, h) = self.renderer.screen_dimensions();
                    let aspect = w as f32 / h as f32;
                    let vp = (perspective(cam.fov, aspect, 0.1, 1000.0)
                        * look_at(&Vec3::ZERO, &cam_front, &cam_up))
                        .inverse()
                        .to_cols_array();
                    [
                        [vp[0], vp[1], vp[2], vp[3]],
                        [vp[4], vp[5], vp[6], vp[7]],
                        [vp[8], vp[9], vp[10], vp[11]],
                        [vp[12], vp[13], vp[14], vp[15]],
                    ]
                },
                prev_view_proj: {
                    #[cfg(feature = "wgpu")]
                    {
                        self.renderer.prev_view_proj
                    }
                    #[cfg(not(feature = "wgpu"))]
                    {
                        [[0.0; 4]; 4]
                    }
                },
                gi_quality,
                gi_debug_mode,
                dir_light_dir: dir_light.direction,
                dir_light_color: [
                    dir_light.color[0] / 255.0,
                    dir_light.color[1] / 255.0,
                    dir_light.color[2] / 255.0,
                ],
                dir_light_intensity: dir_light.intensity,
                sky_occlusion: 0.0,
                gi_mode,
                dof_aperture,
                dof_focus_dist,
                dof_enable,
                atmos,
                atmosphere: if atmosphere && have_atmos { 1 } else { 0 },
            };
            #[cfg(feature = "wgpu")]
            self.renderer.update_scene_data(
                &gpu_objects,
                &gpu_triangles,
                &bvh_nodes,
                &tri_bvh_nodes,
                &gpu_materials,
                &[] as &[crate::scene::object::GpuCustomMaterial],
                &[] as &[String],
                &[] as &[(String, String)],
                &tex_handles,
            );
            #[cfg(not(feature = "wgpu"))]
            self.renderer
                .update_scene_data(&gpu_objects, &gpu_triangles, &bvh_nodes, &tri_bvh_nodes);
            #[cfg(feature = "wgpu")]
            {
                use crate::components::components::Transform;

                let (w, h) = self.renderer.screen_dimensions();
                let aspect = w as f32 / h as f32;
                let view_mat = look_at(&Vec3::ZERO, &cam_front, &cam_up);
                let proj_mat = perspective(cam.fov, aspect, 0.1, 1000.0);

                let mut pbr_meshes = Vec::new();
                for (_e, transform, mesh, mat) in
                    self.world.query3::<Transform, MeshHandle, PbrMaterial>()
                {
                    let model = Mat4::from_scale_rotation_translation(
                        Vec3::from(transform.size),
                        Quat::from_xyzw(
                            transform.orientation[0],
                            transform.orientation[1],
                            transform.orientation[2],
                            transform.orientation[3],
                        ),
                        Vec3::from(transform.position) - cam_pos,
                    );
                    let mvp = (proj_mat * view_mat * model).to_cols_array_2d();
                    pbr_meshes.push(PbrRenderData {
                        mesh: mesh.clone(),
                        material: mat.clone(),
                        mvp,
                        model: model.to_cols_array_2d(),
                    });
                }

                let mut sprite_batches = Vec::new();
                for (_e, transform, sprite) in self.world.query2::<Transform, Sprite3D>() {
                    let pos = Vec3::from(transform.position) - cam_pos;
                    let mut right = Vec3::X;
                    let mut up_v = Vec3::Y;
                    if self.is_2d {
                        if !sprite.facing_camera {
                            let angle =
                                2.0 * transform.orientation[2].atan2(transform.orientation[3]);
                            let q = Quat::from_rotation_z(angle);
                            right = q * right;
                            up_v = q * up_v;
                        }
                    } else if sprite.facing_camera {
                        right = cam_right;
                        up_v = cam_up;
                    } else {
                        let q = Quat::from_xyzw(
                            transform.orientation[0],
                            transform.orientation[1],
                            transform.orientation[2],
                            transform.orientation[3],
                        );
                        right = q * right;
                        up_v = q * up_v;
                    }
                    right = right.normalize() * sprite.size[0] * transform.size[0] * 0.5;
                    up_v = up_v.normalize() * sprite.size[1] * transform.size[1] * 0.5;
                    let p0 = pos - right - up_v;
                    let p1 = pos + right - up_v;
                    let p2 = pos - right + up_v;
                    let p3 = pos + right + up_v;
                    let verts = [
                        [p0.x, p0.y, p0.z, 0.0, 0.0],
                        [p2.x, p2.y, p2.z, 0.0, 1.0],
                        [p1.x, p1.y, p1.z, 1.0, 0.0],
                        [p2.x, p2.y, p2.z, 0.0, 1.0],
                        [p3.x, p3.y, p3.z, 1.0, 1.0],
                        [p1.x, p1.y, p1.z, 1.0, 0.0],
                    ];
                    sprite_batches.push(SpriteRenderData {
                        vertices: verts,
                        texture: sprite.texture.view.clone(),
                        double_sided: sprite.double_sided,
                    });
                }
                let paint_jobs = self
                    .egui_ctx
                    .tessellate(shapes, self.egui_ctx.pixels_per_point());
                #[cfg(feature = "use_epi")]
                self.renderer.render(
                    &render_params,
                    &sprite_batches,
                    &pbr_meshes,
                    Some((&mut self.egui_renderer, &paint_jobs, &textures_delta)),
                );
                #[cfg(not(feature = "use_epi"))]
                self.renderer
                    .render(&render_params, &sprite_batches, &pbr_meshes, None);
            }
            #[cfg(not(feature = "wgpu"))]
            {
                self.renderer.render(&render_params);
            }
            #[cfg(not(feature = "wgpu"))]
            {
                let engine_ptr: *mut Engine = self;
                let sprite_renderer = &mut self.sprite_renderer;
                unsafe {
                    sprite_renderer.update(&mut *engine_ptr, delta);
                }
            }
            #[cfg(not(feature = "wgpu"))]
            {
                self.renderer.capture_screen();
                // Note: Blur regions functionality moved to editor plugin
                let regions: Vec<(i32, i32, i32, i32)> = Vec::new();
                self.renderer.blur_regions(&regions, 10.0);
            }

            #[cfg(all(not(feature = "wgpu"), feature = "use_epi"))]
            {
                let paint_jobs = self
                    .egui_ctx
                    .tessellate(shapes, self.egui_ctx.pixels_per_point());
                self.egui_renderer
                    .paint_jobs(None, textures_delta, paint_jobs);

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
    }

    pub fn run(&mut self, enable_editor: bool) {
        self.run_with_behaviour(enable_editor, EmptyBehaviour);
    }
    pub fn run_default(&mut self) {
        self.run_with_behaviour(true, EmptyBehaviour);
    }
}