use super::Engine;
use crate::components::components::{
    AngularVelocity, Collider, ColliderShape, Material, ObjMesh, ObjectRef, Renderable, Shape,
    Transform,
};
use crate::ecs::Entity;
use crate::ecs::World;
use crate::engine::component_io::{apply_component_data, export_component_data};
use crate::scene::loader::{save_scene, ComponentFile, EntityFile, NodeFile, SceneFile};
use crate::scene::object::{GpuTriangle, Object};
use crate::scene::scene::Scene;
use serde_json::{Map, Value};

#[cfg(feature = "wgpu")]
fn mesh_handle_from_gpu_triangles(
    device: &wgpu::Device,
    name: &str,
    tris: &[GpuTriangle],
) -> Result<crate::gpu::MeshHandle, String> {
    use std::sync::Arc;

    let mut vertices = Vec::with_capacity(tris.len().saturating_mul(3));
    let mut indices = Vec::with_capacity(tris.len().saturating_mul(3));

    for tri in tris {
        let p0 = tri.v0;
        let p1 = [
            tri.v0[0] + tri.e1[0],
            tri.v0[1] + tri.e1[1],
            tri.v0[2] + tri.e1[2],
        ];
        let p2 = [
            tri.v0[0] + tri.e2[0],
            tri.v0[1] + tri.e2[1],
            tri.v0[2] + tri.e2[2],
        ];
        let uv0 = tri.uv0;
        let uv1 = [tri.uv0[0] + tri.duv1[0], tri.uv0[1] + tri.duv1[1]];
        let uv2 = [tri.uv0[0] + tri.duv2[0], tri.uv0[1] + tri.duv2[1]];
        let base = vertices.len() as u32;
        let make_vertex = |pos: [f32; 3], nrm: [f32; 3], uv: [f32; 2]| crate::gpu::Vertex {
            pos,
            nrm,
            tan: [1.0, 0.0, 0.0, 1.0],
            uv,
            joints: [0, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        };
        vertices.push(make_vertex(p0, tri.n0, uv0));
        vertices.push(make_vertex(p1, tri.n1, uv1));
        vertices.push(make_vertex(p2, tri.n2, uv2));
        indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    crate::gpu::GpuMesh::from_cpu(device, name, &vertices, &indices)
        .map(|mesh| crate::gpu::MeshHandle(Arc::new(mesh)))
        .map_err(|e| e.to_string())
}

#[cfg(feature = "wgpu")]
fn pbr_material_from_obj_desc(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    obj_path: &str,
    desc: &crate::rendering::resource::ObjMaterialDesc,
) -> crate::materials::PbrMaterial {
    use std::sync::Arc;
    use crate::gpu::{GpuTexture, TextureHandle};
    use crate::materials::{PbrMaterial, MATERIAL_TAG_CAN_USE_PROBE, MATERIAL_TAG_RASTER_ONLY};

    let base_color_tex = desc.base_color_texture.as_ref().and_then(|path| {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                match GpuTexture::from_rgba8(
                    device,
                    queue,
                    &rgba,
                    w,
                    h,
                    true,
                    &format!("obj:{}:{}", obj_path, desc.name),
                ) {
                    Ok(tex) => Some(TextureHandle(Arc::new(tex))),
                    Err(err) => {
                        eprintln!("OBJ material '{}' texture upload failed for {:?}: {}", desc.name, path, err);
                        None
                    }
                }
            }
            Err(err) => {
                eprintln!("OBJ material '{}' texture load failed for {:?}: {}", desc.name, path, err);
                None
            }
        }
    });

    // OBJ meshes are rendered by the opaque raster mesh path. Keep the
    // authored opacity on `opacity`, but keep base_color.a opaque so it cannot
    // accidentally become the G-buffer visibility mask. This avoids Sponza-like
    // MTL files with `d 0.0` disappearing in the compose pass.
    let mut base_color = desc.base_color;
    base_color[3] = 1.0;

    PbrMaterial {
        name: format!("obj:{}:{}", obj_path, desc.name),
        base_color,
        metallic: desc.metallic,
        roughness: desc.roughness,
        emissive: desc.emissive,
        specular_f0: [0.0; 3],
        ior: 1.5,
        opacity: desc.opacity,
        base_color_tex,
        metallic_roughness_tex: None,
        normal_tex: None,
        occlusion_tex: None,
        emissive_tex: None,
        fallback_tags: MATERIAL_TAG_CAN_USE_PROBE | MATERIAL_TAG_RASTER_ONLY,
    }
}

impl Engine {
    pub fn spawn_empty(&mut self, name: &str) -> Entity {
        let entity = self.world.spawn();
        self.world.insert(
            entity,
            crate::components::components::Metadata {
                name: name.into(),
                tags: Vec::new(),
            },
        );
        entity
    }

    pub fn spawn_object(&mut self, object: Object) {
        self.scene.add_object(object);
        let entity = self.world.spawn();
        let object_id = (self.scene.objects.len() - 1) as u32;
        self.world.insert(entity, ObjectRef { id: object_id });
        self.world.insert(
            entity,
            crate::components::components::Metadata {
                name: format!("Object{}", object_id),
                tags: Vec::new(),
            },
        );
        if let Some(obj) = self.scene.objects.last() {
            // Combine the object's intrinsic size with its scale so the
            // collider matches the rendered geometry.
            let mut final_size = obj.size;
            for i in 0..3 {
                final_size[i] *= obj.scale[i];
            }
            self.world.insert(
                entity,
                Transform {
                    position: obj.position,
                    orientation: obj.orientation,
                    size: final_size,
                },
            );
            self.world.insert(
                entity,
                Renderable {
                    color: obj.color,
                    roughness: obj.roughness,
                    emission: obj.emission,
                    is_mesh: obj.is_mesh,
                    triangle_start_idx: obj.triangle_start_idx as u32,
                    triangle_count: obj.triangle_count as u32,
                },
            );
            let shape = Shape {
                is_cube: obj.is_cube,
                radius: obj.radius,
            };

            let mut collider = Collider::default();
            if obj.is_mesh || shape.is_cube {
                collider.shape = ColliderShape::Cube;
                collider.size = final_size;
            } else {
                collider.shape = ColliderShape::Sphere;
                collider.size = [shape.radius * 2.0, shape.radius * 2.0, shape.radius * 2.0];
            }
            self.world.insert(entity, collider);
            self.world.insert(
                entity,
                Material {
                    is_glass: obj.is_glass,
                    specular_f0: obj.specular_f0.into(),
                    ior: obj.ior,
                },
            );
            self.world.insert(
                entity,
                AngularVelocity {
                    angular_velocity: obj.angular_velocity,
                    angular_acceleration: obj.angular_acceleration,
                },
            );
            self.world.insert(entity, shape);
        }
        self.core.register_object_entity(object_id, entity);
        #[cfg(feature = "wgpu")]
        self.invalidate_material_cache();
    }

    /// Spawn an [`Object`] and return an [`Actor`] wrapper for the created entity.
    pub fn spawn_object_as_actor(&mut self, object: Object) -> Option<super::Actor<'_>> {
        self.spawn_object(object);
        let id = self.scene.objects.len() - 1;
        self.core
            .find_entity_by_object_id(id as u32)
            .map(|e| super::Actor::new(self, e))
    }

    /// Instantiate a [`Prefab`] and return the created [`Actor`].
    pub fn instantiate_prefab(
        &mut self,
        prefab: super::prefab::Prefab,
    ) -> Option<super::Actor<'_>> {
        let mut first = None;
        let scene = prefab.scene;
        for node in scene.nodes {
            let mut obj = Object::default();
            obj.position = node.position;
            obj.color = node.color;
            obj.size = node.size;
            obj.scale = node.scale;
            obj.is_cube = node.is_cube;
            obj.is_static = true;
            obj.orientation = [0.0, 0.0, 0.0, 1.0];
            // if obj.is_cube {
            //     self.spawn_cube(obj);
            // } else {
            //     self.spawn_sphere(obj, 3);
            // }
            self.spawn_object(obj);
            let index = self.scene.objects.len() - 1;
            let entity = self.core.find_entity_by_object_id(index as u32)?;
            if first.is_none() {
                first = Some(entity);
            }
            if let Some(meta) = self
                .world
                .get_mut::<crate::components::components::Metadata>(entity)
            {
                meta.name = node.name;
                meta.tags = node.tags;
            }
            for comp in node.components {
                let key = comp.name;
                let data = comp.data;
                if let Some(factory) = self.component_factories.get(&key) {
                    factory(entity, self, &data);
                } else if self.generated_components.contains(&key) {
                    self.add_generated_component(entity, &key);
                    if let Some(c) = self.get_generated_component_mut(entity, &key) {
                        apply_component_data(c, &data);
                    }
                }
            }
        }
        first.map(|e| super::Actor::new(self, e))
    }

    pub fn spawn_mesh_object(&mut self, path: &str, object: Object) -> Result<(), String> {
        let tris = crate::rendering::resource::load_obj_file(path)?;
        self.spawn_with_triangles(object, tris);
        let object_id = (self.scene.objects.len() - 1) as u32;
        if let Some(entity) = self.core.find_entity_by_object_id(object_id) {
            self.world.insert(
                entity,
                crate::components::components::ObjMesh {
                    path: path.to_string(),
                    loaded: true,
                    loaded_path: path.to_string(),
                    // spawn_mesh_object is the explicit legacy path that creates
                    // ray-traceable triangle geometry. Editor-added ObjMesh
                    // components default to raster-only for performance.
                    raytrace: true,
                    submesh_entities: Vec::new(),
                },
            );
        }
        Ok(())
    }

    pub fn spawn_with_triangles(&mut self, mut object: Object, mut tris: Vec<GpuTriangle>) {
        // Center mesh geometry around the origin so rotation/scale operate about
        // the object's true center and the top-level BVH encloses the mesh
        // correctly even when source data is offset in space.
        let (min_b, max_b) = crate::scene::tri_bvh::mesh_bounds(&tris);
        let center = [
            (min_b[0] + max_b[0]) * 0.5,
            (min_b[1] + max_b[1]) * 0.5,
            (min_b[2] + max_b[2]) * 0.5,
        ];
        for t in &mut tris {
            t.v0[0] -= center[0];
            t.v0[1] -= center[1];
            t.v0[2] -= center[2];
            if t.material_index == u32::MAX {
                t.material_index = object.material_index;
            }
        }

        // Recompute bounds after centering to determine the unscaled size
        let (min_b, max_b) = crate::scene::tri_bvh::mesh_bounds(&tris);
        object.size = [
            max_b[0] - min_b[0],
            max_b[1] - min_b[1],
            max_b[2] - min_b[2],
        ];

        // Add triangles and their BVH nodes to the scene
        let start = self.scene.triangles.len();
        let count = tris.len();
        self.scene.add_triangles(tris.clone());
        let mut bvh_nodes = crate::scene::tri_bvh::build_bvh(&tris);
        let bvh_start = self.scene.tri_bvh_nodes.len();
        crate::scene::tri_bvh::offset_nodes(&mut bvh_nodes, bvh_start as i32);
        let bvh_count = bvh_nodes.len();
        self.scene.add_tri_bvh_nodes(bvh_nodes);

        object.is_mesh = true;
        object.triangle_start_idx = start;
        object.triangle_count = count;
        object.tri_bvh_start = bvh_start;
        object.tri_bvh_count = bvh_count;
        self.spawn_object(object);
    }

    pub fn spawn_cube(&mut self, object: Object) {
        let tris = crate::rendering::resource::generate_cube_triangles(object.size);
        self.spawn_with_triangles(object, tris);
    }

    pub fn spawn_sphere(&mut self, mut object: Object, smoothness: u32) {
        let tris = crate::rendering::resource::generate_sphere_triangles(object.radius, smoothness);
        object.size = [object.radius * 2.0; 3];
        object.is_cube = false;
        self.spawn_with_triangles(object, tris);
    }

    /// Load a [`SceneFile`] already parsed in memory.
    pub fn load_scene(&mut self, scene: SceneFile) -> Result<(), Box<dyn std::error::Error>> {
        for node in scene.nodes {
            let mut obj = Object::default();
            obj.position = node.position;
            obj.color = node.color;
            obj.size = node.size;
            obj.scale = node.scale;
            obj.is_cube = node.is_cube;
            obj.is_static = true;
            obj.orientation = [0.0, 0.0, 0.0, 1.0];
            // if obj.is_cube {
            //     self.spawn_cube(obj);
            // } else {
            //     self.spawn_sphere(obj, 3);
            // }
            self.spawn_object(obj);
            let object_id = (self.scene.objects.len() - 1) as u32;
            let entity = self.core.find_entity_by_object_id(object_id).unwrap();
            if let Some(meta) = self
                .world
                .get_mut::<crate::components::components::Metadata>(entity)
            {
                meta.name = node.name.clone();
                meta.tags = node.tags.clone();
            }
            for comp in node.components {
                let key = comp.name;
                let data = comp.data;
                if let Some(factory) = self.component_factories.get(&key) {
                    factory(entity, self, &data);
                } else if self.generated_components.contains(&key) {
                    self.add_generated_component(entity, &key);
                    if let Some(c) = self.get_generated_component_mut(entity, &key) {
                        apply_component_data(c, &data);
                    }
                } else {
                    println!("Unknown component {} on node: {}", key, node.name);
                }
            }
        }

        for ent in scene.entities {
            let entity = self.spawn_empty(&ent.name);
            if let Some(meta) = self
                .world
                .get_mut::<crate::components::components::Metadata>(entity)
            {
                meta.tags = ent.tags.clone();
            }
            for comp in ent.components {
                let key = comp.name;
                let data = comp.data;
                if let Some(factory) = self.component_factories.get(&key) {
                    factory(entity, self, &data);
                } else if self.generated_components.contains(&key) {
                    self.add_generated_component(entity, &key);
                    if let Some(c) = self.get_generated_component_mut(entity, &key) {
                        apply_component_data(c, &data);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn load_scene_from_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let scene = crate::scene::loader::load_scene(path)?;
        self.load_scene(scene)
    }

    pub fn save_scene_to_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure scene object data reflects the latest world transforms so
        // saved files capture edits made through the editor.
        self.scene.rebuild_from_world(&mut self.world);
        let mut nodes = Vec::new();
        let mut entities = Vec::new();
        for idx in 0..self.scene.objects.len() {
            let obj = self.scene.objects[idx];
            let mut components = Vec::new();
            if let Some(entity) = self.core.find_entity_by_object_id(idx as u32) {
                let comp_names = self.list_components(idx);
                for name in comp_names {
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
            let mut components = Vec::new();
            for cname in self.list_components_entity(entity) {
                if cname == "Metadata" || cname == "ObjectRef" {
                    continue;
                }
                if let Some(comp) = self.access_component_mut(entity, &cname) {
                    components.push(ComponentFile {
                        name: cname,
                        data: export_component_data(comp),
                    });
                }
            }
            entities.push(EntityFile {
                name,
                tags,
                components,
            });
        }
        let scene_file = SceneFile { nodes, entities };
        save_scene(path, &scene_file)?;
        Ok(())
    }

    /// Remove all objects and entities from the current scene and world.
    pub fn clear_scene(&mut self) {
        let entities = self.world.entities().to_vec();
        for e in entities {
            self.delete_entity(e);
        }
        self.scene = Scene::new();
        self.core.object_entity_map.clear();
        self.world = World::new();
        self.scene_events = crate::events::SceneEvents::new();
    }

    pub fn update_obj_meshes(&mut self) {
        #[cfg(feature = "wgpu")]
        let device = self.renderer.device().clone();
        #[cfg(feature = "wgpu")]
        let queue = self.renderer.queue().clone();
        #[cfg(feature = "wgpu")]
        let mut pending_mesh_handles: Vec<(Entity, crate::gpu::MeshHandle)> = Vec::new();
        #[cfg(feature = "wgpu")]
        let mut pending_materials: Vec<(Entity, crate::materials::PbrMaterial)> = Vec::new();
        #[cfg(feature = "wgpu")]
        let mut pending_child_spawns: Vec<(
            Entity,
            crate::components::components::Transform,
            crate::gpu::MeshHandle,
            crate::materials::PbrMaterial,
            String,
        )> = Vec::new();
        let mut pending_delete_entities: Vec<Entity> = Vec::new();
        #[cfg(feature = "wgpu")]
        let mut pending_submesh_owner_updates: Vec<Entity> = Vec::new();

        for (entity, mesh, obj_ref, render, shape) in self
            .world
            .query4_mut::<crate::components::components::ObjMesh, crate::components::components::ObjectRef, crate::components::components::Renderable, crate::components::components::Shape>()
        {
            let path = mesh.path.trim().to_string();
            let needs_reload = !path.is_empty() && (!mesh.loaded || mesh.loaded_path != path);
            if !needs_reload {
                continue;
            }

            pending_delete_entities.extend(mesh.submesh_entities.iter().copied());
            mesh.submesh_entities.clear();

            match crate::rendering::resource::load_obj_file_with_materials(&path) {
                Ok(mut submeshes) => {
                    let total_triangles: usize = submeshes.iter().map(|s| s.triangles.len()).sum();
                    eprintln!(
                        "ObjMesh loaded '{}': submeshes={}, triangles={}",
                        path,
                        submeshes.len(),
                        total_triangles
                    );
                    if total_triangles == 0 {
                        eprintln!("ObjMesh '{}' loaded zero triangles", path);
                        mesh.loaded = true;
                        mesh.loaded_path = path;
                        continue;
                    }

                    let mut all_tris = Vec::with_capacity(total_triangles);
                    for sm in &submeshes {
                        all_tris.extend(sm.triangles.iter().copied());
                    }
                    let (bmin_src, bmax_src) = crate::scene::tri_bvh::mesh_bounds(&all_tris);
                    let center = [
                        (bmin_src[0] + bmax_src[0]) * 0.5,
                        (bmin_src[1] + bmax_src[1]) * 0.5,
                        (bmin_src[2] + bmax_src[2]) * 0.5,
                    ];
                    for sm in &mut submeshes {
                        for t in &mut sm.triangles {
                            t.v0[0] -= center[0];
                            t.v0[1] -= center[1];
                            t.v0[2] -= center[2];
                        }
                    }
                    all_tris.clear();
                    for sm in &submeshes {
                        all_tris.extend(sm.triangles.iter().copied());
                    }
                    let (bmin, bmax) = crate::scene::tri_bvh::mesh_bounds(&all_tris);
                    let mesh_size = [
                        (bmax[0] - bmin[0]).max(0.001),
                        (bmax[1] - bmin[1]).max(0.001),
                        (bmax[2] - bmin[2]).max(0.001),
                    ];
                    let mesh_radius = 0.5 * mesh_size[0].max(mesh_size[1]).max(mesh_size[2]);

                    #[cfg(feature = "wgpu")]
                    {
                        for (idx, sm) in submeshes.iter().enumerate() {
                            match mesh_handle_from_gpu_triangles(
                                &device,
                                &format!("{}#{}", path, sm.name),
                                &sm.triangles,
                            ) {
                                Ok(handle) => {
                                    let mat = pbr_material_from_obj_desc(&device, &queue, &path, &sm.material);
                                    if idx == 0 {
                                        pending_mesh_handles.push((entity, handle));
                                        pending_materials.push((entity, mat));
                                    } else {
                                        pending_child_spawns.push((
                                            entity,
                                            crate::components::components::Transform::default(),
                                            handle,
                                            mat,
                                            format!("{}:{}", path, sm.name),
                                        ));
                                    }
                                }
                                Err(err) => eprintln!("ObjMesh '{}' raster upload failed for submesh '{}': {}", path, sm.name, err),
                            }
                        }
                    }

                    render.is_mesh = true;
                    shape.is_cube = false;
                    shape.radius = mesh_radius;

                    let (triangle_start_idx, triangle_count, tri_bvh_start, tri_bvh_count) =
                        if mesh.raytrace {
                            let start = self.scene.triangles.len();
                            self.scene.add_triangles(all_tris.clone());

                            let mut bvh_nodes = crate::scene::tri_bvh::build_bvh(&all_tris);
                            let b_start = self.scene.tri_bvh_nodes.len();
                            crate::scene::tri_bvh::offset_nodes(&mut bvh_nodes, b_start as i32);
                            let b_count = bvh_nodes.len();
                            self.scene.add_tri_bvh_nodes(bvh_nodes);

                            let count = all_tris.len();
                            render.triangle_start_idx = start as u32;
                            render.triangle_count = count as u32;
                            (start, count, b_start, b_count)
                        } else {
                            render.triangle_start_idx = 0;
                            render.triangle_count = 0;
                            (0, 0, 0, 0)
                        };

                    if let Some(obj) = self.scene.objects.get_mut(obj_ref.id as usize) {
                        obj.is_mesh = true;
                        obj.is_cube = false;
                        obj.size = mesh_size;
                        obj.radius = mesh_radius;
                        obj.triangle_start_idx = triangle_start_idx;
                        obj.triangle_count = triangle_count;
                        obj.tri_bvh_start = tri_bvh_start;
                        obj.tri_bvh_count = tri_bvh_count;
                    }

                    self.scene.bvh_dirty = true;
                    mesh.loaded = true;
                    mesh.loaded_path = path;
                    #[cfg(feature = "wgpu")]
                    pending_submesh_owner_updates.push(entity);
                }
                Err(err) => {
                    eprintln!("ObjMesh load failed for '{}': {}", path, err);
                    mesh.loaded = false;
                    mesh.loaded_path.clear();
                }
            }
        }

        for entity in pending_delete_entities {
            #[cfg(feature = "wgpu")]
            self.world.remove::<crate::gpu::MeshHandle>(entity);
            #[cfg(feature = "wgpu")]
            self.world.remove::<crate::materials::PbrMaterial>(entity);
            self.world.remove::<crate::components::components::Transform>(entity);
            self.world.remove::<crate::components::components::GlobalTransform>(entity);
            self.world.remove::<crate::components::components::Parent>(entity);
            self.world.remove::<crate::components::components::Metadata>(entity);
            self.world.delete_entity(entity);
        }

        #[cfg(feature = "wgpu")]
        for (entity, handle) in pending_mesh_handles {
            self.world.insert(entity, handle);
        }

        #[cfg(feature = "wgpu")]
        for (entity, mat) in pending_materials {
            self.world.insert(entity, mat);
        }

        #[cfg(feature = "wgpu")]
        {
            let mut spawned_by_parent: std::collections::HashMap<Entity, Vec<Entity>> = std::collections::HashMap::new();
            for (parent, local_transform, handle, mat, name) in pending_child_spawns {
                let child = self.world.spawn();
                self.world.insert(child, crate::components::components::Metadata {
                    name,
                    tags: vec!["obj_submesh".to_string()],
                });
                self.world.insert(child, local_transform);
                self.world.insert(child, crate::components::components::Parent { entity: parent });
                self.world.insert(child, handle);
                self.world.insert(child, mat);
                spawned_by_parent.entry(parent).or_default().push(child);
            }

            for parent in pending_submesh_owner_updates {
                if let Some(mesh) = self.world.get_mut::<crate::components::components::ObjMesh>(parent) {
                    mesh.submesh_entities = spawned_by_parent.remove(&parent).unwrap_or_default();
                }
            }
        }
    }

}
