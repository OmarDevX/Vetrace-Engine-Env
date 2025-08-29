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
        for (_e, mesh, obj_ref, render, shape) in self
            .world
            .query4_mut::<crate::components::components::ObjMesh, crate::components::components::ObjectRef, crate::components::components::Renderable, crate::components::components::Shape>()
        {
            if !mesh.loaded && !mesh.path.is_empty() {
                if let Ok(mut tris) = crate::rendering::resource::load_obj_file(&mesh.path) {
                    // Center the mesh geometry about the origin to keep BVH and
                    // transforms stable even when source OBJ data is offset.
                    let (bmin, bmax) = crate::scene::tri_bvh::mesh_bounds(&tris);
                    let center = [
                        (bmin[0] + bmax[0]) * 0.5,
                        (bmin[1] + bmax[1]) * 0.5,
                        (bmin[2] + bmax[2]) * 0.5,
                    ];
                    for t in &mut tris {
                        t.v0[0] -= center[0];
                        t.v0[1] -= center[1];
                        t.v0[2] -= center[2];
                    }
                    let (bmin, bmax) = crate::scene::tri_bvh::mesh_bounds(&tris);

                    let start = self.scene.triangles.len();
                    self.scene.add_triangles(tris.clone());

                    // Build a per-mesh triangle BVH and determine the mesh bounds
                    let mut bvh_nodes = crate::scene::tri_bvh::build_bvh(&tris);
                    let b_start = self.scene.tri_bvh_nodes.len();
                    crate::scene::tri_bvh::offset_nodes(&mut bvh_nodes, b_start as i32);
                    let b_count = bvh_nodes.len();
                    self.scene.add_tri_bvh_nodes(bvh_nodes);

                    // Record triangle range for the object and compute its unscaled size
                    let count = tris.len();
                    render.is_mesh = true;
                    render.triangle_start_idx = start as u32;
                    render.triangle_count = count as u32;
                    shape.is_cube = false;
                    shape.radius = 0.5
                        * (bmax[0] - bmin[0])
                            .max(bmax[1] - bmin[1])
                            .max(bmax[2] - bmin[2]);
                    if let Some(obj) = self.scene.objects.get_mut(obj_ref.id as usize) {
                        obj.is_mesh = true;
                        obj.triangle_start_idx = start;
                        obj.triangle_count = count;
                        obj.tri_bvh_start = b_start;
                        obj.tri_bvh_count = b_count;
                        obj.size = [
                            bmax[0] - bmin[0],
                            bmax[1] - bmin[1],
                            bmax[2] - bmin[2],
                        ];
                    }

                    // Mark BVH dirty so top-level structure rebuilds with new bounds
                    self.scene.bvh_dirty = true;
                    mesh.loaded = true;
                }
            }
        }
    }
}
