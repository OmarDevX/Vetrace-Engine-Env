use crate::math::vec3;
use crate::materials::PbrMaterial;
use crate::scene::object::{GpuObject, GpuTriangle, GpuAtmosphere, Object};

pub struct Scene {
    pub objects: Vec<Object>,
    pub gpu_objects: Vec<GpuObject>,
    pub triangles: Vec<GpuTriangle>,
    pub tri_bvh_nodes: Vec<crate::scene::tri_bvh::GpuTriBvhNode>,
    pub bvh_nodes: Vec<crate::scene::bvh::GpuBvhNode>,
    pub materials: Vec<PbrMaterial>,
    pub bvh_dirty: bool,
    pub atmospheres: Vec<GpuAtmosphere>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            objects: vec![],
            gpu_objects: vec![],
            triangles: vec![],
            tri_bvh_nodes: vec![],
            bvh_nodes: vec![],
            materials: vec![],
            bvh_dirty: true,
            atmospheres: vec![],
        }
    }

    pub fn update(
        &mut self,
        world: &crate::ecs::World,
        map: &std::collections::HashMap<u32, crate::ecs::Entity>,
    ) {
        let delta_time = 0.016;
        let mut all = self.objects.clone();
        for (i, obj) in self.objects.iter_mut().enumerate() {
            if let Some(entity) = map.get(&(i as u32)) {
                if world
                    .get::<crate::components::components::RigidBody3D>(*entity)
                    .is_some()
                {
                    continue;
                }
            }
            obj.update(delta_time, &mut all);
        }
        self.gpu_objects = self.objects.iter().map(|o| o.to_gpu()).collect();
    }

    pub fn rebuild_from_world(&mut self, world: &mut crate::ecs::World) {
        crate::systems::hierarchy::update_global_transforms(world);
        let previous = self.objects.clone();
        self.gpu_objects.clear();
        self.atmospheres.clear();
        for &entity in world.entities().to_vec().iter() {
            let (transform, material, renderable, collider) = match (
                world.get::<crate::components::components::Transform>(entity),
                world.get::<crate::components::components::Material>(entity),
                world.get::<crate::components::components::Renderable>(entity),
                world.get::<crate::components::components::Collider>(entity),
            ) {
                (Some(t), Some(m), Some(r), Some(c)) => (t, m, r, c),
                _ => continue,
            };
            let obj_ref = world.get::<crate::components::components::ObjectRef>(entity);
            let global = world
                .get::<crate::components::components::GlobalTransform>(entity)
                .cloned();
            let (pos, ori) = if let Some(g) = global {
                (g.position, g.orientation)
            } else {
                (transform.position, transform.orientation)
            };
            let mut obj_size = transform.size;
            let emission = renderable.emission;
            let mat_ior = material.ior;
            let spec_f0 = material.specular_f0.into();
            let velocity = world
                .get::<crate::components::components::Velocity>(entity)
                .cloned()
                .unwrap_or_default();
            let rigid_body = world.get::<crate::components::components::RigidBody3D>(entity);
            let angular = world
                .get::<crate::components::components::AngularVelocity>(entity)
                .cloned()
                .unwrap_or_default();
            let mut obj_scale = [1.0; 3];
            if let Some(r) = obj_ref {
                if let Some(obj) = self.objects.get_mut(r.id as usize) {
                    let prev = previous.get(r.id as usize).copied();
                    obj.position = pos;
                    obj.orientation = ori;
                    // Derive per-axis scale factors from the world-space
                    // transform size so triangle vertices are scaled
                    // correctly relative to the object's original mesh
                    // dimensions.
                    for i in 0..3 {
                        obj.scale[i] = if obj.size[i] != 0.0 {
                            transform.size[i] / obj.size[i]
                        } else {
                            1.0
                        };
                    }
                    obj.color = renderable.color;
                    obj.roughness = renderable.roughness;
                    obj.emission = renderable.emission;
                    obj.is_mesh = renderable.is_mesh;
                    obj.triangle_start_idx = renderable.triangle_start_idx as usize;
                    obj.triangle_count = renderable.triangle_count as usize;
                    obj.is_glass = material.is_glass;
                    obj.specular_f0 = material.specular_f0.into();
                    obj.ior = material.ior;
                    obj.is_cube = collider.is_cube;
                    obj.radius = collider.radius;
                    obj.velocity = velocity.velocity;
                    // The `Velocity` component already stores the current
                    // acceleration including gravity, so copy it directly
                    // without adding gravity again.
                    obj.acceleration = velocity.acceleration;
                    obj.angular_velocity = angular.angular_velocity;
                    obj.angular_acceleration = angular.angular_acceleration;

                    if let Some(p) = prev {
                        if p.position != obj.position
                            || p.scale != obj.scale
                            || p.radius != obj.radius
                            || p.orientation != obj.orientation
                        {
                            self.bvh_dirty = true;
                        }
                    } else {
                        self.bvh_dirty = true;
                    }

                    obj_size = obj.size;
                    obj_scale = obj.scale;
                }
            }

            let is_cube = collider.is_cube as u32;
            let is_mesh = renderable.is_mesh as u32;
            let radius = collider.radius;
            let (tri_start, tri_count) = if renderable.is_mesh {
                (renderable.triangle_start_idx, renderable.triangle_count)
            } else {
                (0, 0)
            };
            let gpu_object = GpuObject {
                position: pos,
                orientation: ori,
                size: obj_size,
                scale: obj_scale,
                material_index: 0,
                radius,
                is_glass: material.is_glass as u32,
                is_cube,
                is_mesh,
                triangle_start_idx: if renderable.is_mesh {
                    renderable.triangle_start_idx
                } else {
                    tri_start
                },
                triangle_count: if renderable.is_mesh {
                    renderable.triangle_count
                } else {
                    tri_count
                },
                tri_bvh_start: 0,
                tri_bvh_count: 0,
                ..GpuObject::default()
            };
            self.gpu_objects.push(gpu_object);
            if let Some(atmo) = world.get::<crate::components::components::Atmosphere>(entity) {
                let center = pos;
                let g_atmo = GpuAtmosphere {
                    center_radius: [center[0], center[1], center[2], atmo.planet_radius],
                    atmo_g_height: [atmo.atmo_radius, atmo.g, atmo.height_ray, atmo.height_mie],
                    ray_beta: [atmo.ray_beta.x, atmo.ray_beta.y, atmo.ray_beta.z, 0.0],
                    mie_beta: [atmo.mie_beta.x, atmo.mie_beta.y, atmo.mie_beta.z, 0.0],
                    ambient_beta: [atmo.ambient_beta.x, atmo.ambient_beta.y, atmo.ambient_beta.z, 0.0],
                    absorption_beta: [atmo.absorption_beta.x, atmo.absorption_beta.y, atmo.absorption_beta.z, 0.0],
                    absorb_params: [
                        atmo.height_absorption,
                        atmo.absorption_falloff,
                        atmo.primary_steps as f32,
                        atmo.light_steps as f32,
                    ],
                };
                self.atmospheres.push(g_atmo);
            }

            let _ = transform;
            let _ = material;
            let _ = renderable;
            let _ = collider;

            if let Some(pbr) = world.get_mut::<crate::materials::PbrMaterial>(entity) {
                pbr.emissive = [emission; 3];
                pbr.ior = mat_ior;
                pbr.specular_f0 = spec_f0;
            }
        }
    }

    pub fn get_gpu_buffers(&self) -> (&[GpuObject], &[GpuTriangle]) {
        (&self.gpu_objects, &self.triangles)
    }

    pub fn get_gpu_atmospheres(&self) -> &[GpuAtmosphere] {
        &self.atmospheres
    }

    pub fn add_object(&mut self, object: Object) {
        self.objects.push(object);
        self.gpu_objects = self.objects.iter().map(|o| o.to_gpu()).collect();
        self.bvh_dirty = true;
    }

    pub fn remove_object(&mut self, index: usize) {
        if index >= self.objects.len() {
            return;
        }

        let obj = self.objects.remove(index);
        if obj.is_mesh && obj.triangle_count > 0 {
            let start = obj.triangle_start_idx;
            let end = start + obj.triangle_count;
            if end <= self.triangles.len() {
                self.triangles.drain(start..end);
                for o in &mut self.objects {
                    if o.triangle_start_idx > start {
                        o.triangle_start_idx -= obj.triangle_count;
                    }
                }
            }
            let b_start = obj.tri_bvh_start;
            let b_end = b_start + obj.tri_bvh_count;
            if b_end <= self.tri_bvh_nodes.len() {
                self.tri_bvh_nodes.drain(b_start..b_end);
                for o in &mut self.objects {
                    if o.tri_bvh_start > b_start {
                        o.tri_bvh_start -= obj.tri_bvh_count;
                    }
                }
            }
        }

        self.gpu_objects = self.objects.iter().map(|o| o.to_gpu()).collect();
        self.bvh_dirty = true;
    }

    pub fn add_triangles(&mut self, triangles: Vec<GpuTriangle>) {
        self.triangles.extend(triangles);
    }

    pub fn add_tri_bvh_nodes(&mut self, nodes: Vec<crate::scene::tri_bvh::GpuTriBvhNode>) {
        self.tri_bvh_nodes.extend(nodes);
    }

    pub fn get_gpu_objects(&self) -> &[GpuObject] {
        &self.gpu_objects
    }

    pub fn get_gpu_triangles(&self) -> &[GpuTriangle] {
        &self.triangles
    }

    pub fn get_tri_bvh_nodes(&self) -> &[crate::scene::tri_bvh::GpuTriBvhNode] {
        &self.tri_bvh_nodes
    }

    pub fn rebuild_bvh(&mut self) {
        self.bvh_nodes = crate::scene::bvh::build_bvh(&self.objects, &self.triangles);
        self.bvh_dirty = false;
    }

    /// Rebuild the BVH if any object has changed since the last build.
    pub fn ensure_bvh(&mut self) {
        if self.bvh_dirty {
            self.rebuild_bvh();
        }
    }

    pub fn get_bvh_nodes(&self) -> &[crate::scene::bvh::GpuBvhNode] {
        &self.bvh_nodes
    }

    pub fn sync_objects_to_world(
        &self,
        world: &mut crate::ecs::World,
        map: &std::collections::HashMap<u32, crate::ecs::Entity>,
    ) {
        for (i, obj) in self.objects.iter().enumerate() {
            if let Some(entity) = map.get(&(i as u32)) {
                if world
                    .get::<crate::components::components::Parent>(*entity)
                    .is_none()
                {
                    if let Some(t) =
                        world.get_mut::<crate::components::components::Transform>(*entity)
                    {
                        t.position = obj.position;
                        t.orientation = obj.orientation;
                        t.size = obj.scale;
                    }
                }
                if let Some(v) = world.get_mut::<crate::components::components::Velocity>(*entity) {
                    v.velocity = obj.velocity;
                    v.acceleration = obj.acceleration;
                }
                if let Some(a) =
                    world.get_mut::<crate::components::components::AngularVelocity>(*entity)
                {
                    a.angular_velocity = obj.angular_velocity;
                    a.angular_acceleration = obj.angular_acceleration;
                }
            }
        }
    }
}