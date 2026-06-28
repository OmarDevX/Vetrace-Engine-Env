use crate::materials::PbrMaterial;
use crate::math::{vec3, Vec3};
use crate::scene::object::{GpuAtmosphere, GpuObject, GpuTriangle, GpuVolumetricCloud, Object};

pub struct Scene {
    pub objects: Vec<Object>,
    pub gpu_objects: Vec<GpuObject>,
    pub triangles: Vec<GpuTriangle>,
    pub tri_bvh_nodes: Vec<crate::scene::tri_bvh::GpuTriBvhNode>,
    pub bvh_nodes: Vec<crate::scene::bvh::GpuBvhNode>,
    pub materials: Vec<PbrMaterial>,
    pub bvh_dirty: bool,
    pub atmospheres: Vec<GpuAtmosphere>,
    pub clouds: Vec<GpuVolumetricCloud>,
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
            clouds: vec![],
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

    pub fn rebuild_from_world(&mut self, world: &mut crate::ecs::World) -> bool {
        crate::systems::hierarchy::update_global_transforms(world);
        let previous = self.objects.clone();
        let mut materials_changed = false;
        self.gpu_objects.clear();
        self.clouds.clear();
        self.atmospheres.clear();
        for &entity in world.entities().to_vec().iter() {
            let (transform, material, renderable, shape) = match (
                world.get::<crate::components::components::Transform>(entity),
                world.get::<crate::components::components::Material>(entity),
                world.get::<crate::components::components::Renderable>(entity),
                world.get::<crate::components::components::Shape>(entity),
            ) {
                (Some(t), Some(m), Some(r), Some(s)) => (t, m, r, s),
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
                    if renderable.is_mesh {
                        obj.is_cube = false;
                        obj.radius = 0.5 * obj_size[0].max(obj_size[1]).max(obj_size[2]);
                    } else if shape.is_cube {
                        obj.is_cube = true;
                        obj.radius = 0.5 * obj_size[0].max(obj_size[1]).max(obj_size[2]);
                    } else {
                        obj.is_cube = false;
                        obj.radius = shape.radius;
                    }
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
                        if p.color != obj.color
                            || p.roughness != obj.roughness
                            || p.emission != obj.emission
                            || p.is_glass != obj.is_glass
                            || p.specular_f0 != obj.specular_f0
                            || p.ior != obj.ior
                            || p.is_mesh != obj.is_mesh
                            || p.triangle_start_idx != obj.triangle_start_idx
                            || p.triangle_count != obj.triangle_count
                        {
                            materials_changed = true;
                        }
                    } else {
                        self.bvh_dirty = true;
                        materials_changed = true;
                    }

                    obj_size = obj.size;
                    obj_scale = obj.scale;
                }
            }

            let is_cube = if renderable.is_mesh {
                0
            } else {
                shape.is_cube as u32
            };
            let is_mesh = renderable.is_mesh as u32;
            let radius = if renderable.is_mesh {
                0.5 * obj_size[0].max(obj_size[1]).max(obj_size[2])
            } else if shape.is_cube {
                0.5 * obj_size[0].max(obj_size[1]).max(obj_size[2])
            } else {
                shape.radius
            };
            let (tri_start, tri_count) = if renderable.is_mesh {
                (renderable.triangle_start_idx, renderable.triangle_count)
            } else {
                (0, 0)
            };
            // Keep scene-backed GPU objects in scene object ID order.  The raster
            // primitive pass uses instance object_index values to index this buffer,
            // so appending ObjectRef entities in arbitrary ECS entity iteration order
            // makes only certain scene IDs render correctly.  Renderable ECS entities
            // without an ObjectRef are still appended after the ordered scene objects.
            if obj_ref.is_none() {
                self.gpu_objects.push(GpuObject {
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
                });
            }
            if let Some(atmo) = world.get::<crate::components::components::Atmosphere>(entity) {
                let center = pos;
                // Atmosphere distances are already authored in world units with
                // `1 world unit = 1 km`; pack them unchanged for the shader.
                let g_atmo = GpuAtmosphere {
                    center_radius: [center[0], center[1], center[2], atmo.planet_radius],
                    atmo_g_height: [atmo.atmo_radius, atmo.g, atmo.height_ray, atmo.height_mie],
                    ray_beta: [atmo.ray_beta.x, atmo.ray_beta.y, atmo.ray_beta.z, 0.0],
                    mie_beta: [atmo.mie_beta.x, atmo.mie_beta.y, atmo.mie_beta.z, 0.0],
                    ambient_beta: [
                        atmo.ambient_beta.x,
                        atmo.ambient_beta.y,
                        atmo.ambient_beta.z,
                        0.0,
                    ],
                    absorption_beta: [
                        atmo.absorption_beta.x,
                        atmo.absorption_beta.y,
                        atmo.absorption_beta.z,
                        0.0,
                    ],
                    absorb_params: [
                        atmo.absorption_lower_width,
                        atmo.absorption_upper_width,
                        atmo.absorption_density_scale,
                        atmo.primary_steps as f32,
                    ],
                    ozone_params: [
                        atmo.absorption_lower_exp_term,
                        atmo.absorption_lower_exp_scale,
                        atmo.absorption_lower_linear_term,
                        atmo.absorption_lower_constant_term,
                    ],
                    absorption_layer_params: [
                        atmo.absorption_upper_exp_term,
                        atmo.absorption_upper_exp_scale,
                        atmo.absorption_upper_linear_term,
                        atmo.absorption_upper_constant_term,
                    ],
                    multi_scatter_params: [
                        atmo.multi_scatter_strength,
                        atmo.multi_scatter_falloff,
                        atmo.multi_scatter_phase_boost,
                        atmo.multi_scatter_ambient_mix,
                    ],
                };
                self.atmospheres.push(g_atmo);
            }
            if let Some(cloud) = world.get::<crate::components::components::VolumetricCloud>(entity)
            {
                let wind = if cloud.wind_direction.length_squared() > 0.0 {
                    cloud.wind_direction.normalize()
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                };
                let planet_radius = if let Some(atmo) =
                    world.get::<crate::components::components::Atmosphere>(entity)
                {
                    atmo.planet_radius
                } else {
                    radius * obj_scale[0].max(obj_scale[1]).max(obj_scale[2])
                };
                let cloud_base_radius = (planet_radius + cloud.base_height).max(0.001);
                self.clouds.push(GpuVolumetricCloud {
                    center_base_thickness: [pos[0], pos[1], pos[2], cloud_base_radius],
                    coverage_density_noise_phase: [
                        cloud.coverage,
                        cloud.density,
                        cloud.noise_scale,
                        cloud.phase_anisotropy,
                    ],
                    wind_steps: [wind.x, wind.z, cloud.wind_speed, cloud.primary_steps as f32],
                    light_padding: [
                        cloud.cloud_light_steps as f32,
                        cloud.shadow_strength,
                        cloud.planet_shadow_penumbra,
                        cloud.object_shadow_quality.clamp(0, 4) as f32,
                    ],
                    multi_scatter: [
                        cloud.multi_scatter_strength,
                        cloud.multi_scatter_octaves as f32,
                        cloud.multi_scatter_attenuation,
                        cloud.multi_scatter_eccentricity,
                    ],
                    shape_params: [
                        cloud.thickness.max(0.001),
                        cloud.primary_steps as f32,
                        cloud.shape_scale.max(0.001),
                        cloud.detail_scale.max(0.001),
                    ],
                    weather_params: [
                        cloud.weather_scale.max(0.0001),
                        cloud.weather_offset.x,
                        cloud.weather_offset.z,
                        cloud.macro_variation,
                    ],
                    detail_params: [
                        cloud.erosion_strength,
                        cloud.cloud_type,
                        cloud.anvil_strength,
                        cloud.curl_strength,
                    ],
                    lighting_params0: [
                        cloud.forward_anisotropy,
                        cloud.backward_anisotropy,
                        cloud.lobe_blend,
                        cloud.powder_strength,
                    ],
                    lighting_params1: [cloud.silver_lining_strength, 0.0, 0.0, 0.0],
                });
            }

            let _ = transform;
            let _ = material;
            let _ = renderable;

            if let Some(pbr) = world.get_mut::<crate::materials::PbrMaterial>(entity) {
                pbr.emissive = [emission; 3];
                pbr.ior = mat_ior;
                pbr.specular_f0 = spec_f0;
            }
        }

        let mut ordered_gpu_objects: Vec<GpuObject> =
            self.objects.iter().map(|o| o.to_gpu()).collect();
        if !self.gpu_objects.is_empty() {
            ordered_gpu_objects.extend(self.gpu_objects.drain(..));
        }
        self.gpu_objects = ordered_gpu_objects;

        materials_changed
    }

    pub fn get_gpu_buffers(&self) -> (&[GpuObject], &[GpuTriangle]) {
        (&self.gpu_objects, &self.triangles)
    }

    pub fn get_gpu_atmospheres(&self) -> &[GpuAtmosphere] {
        &self.atmospheres
    }

    pub fn get_gpu_clouds(&self) -> &[GpuVolumetricCloud] {
        &self.clouds
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

    /// Calculate a conservative near plane based on the distance to the closest
    /// object's oriented bounding box. Using an OBB instead of a bounding
    /// sphere avoids the pathological case where large but thin meshes (e.g.
    /// walls or sprites) force an extremely small near plane, which in turn
    /// tanks performance when the camera moves indoors or very close to flat
    /// geometry.
    pub fn camera_near_plane(&self, cam_pos: Vec3) -> f32 {
        use glam::Quat;

        let mut min_dist = f32::MAX;
        for obj in &self.objects {
            let center = Vec3::from(obj.position);
            let half = Vec3::from(obj.size) * 0.5;
            let q = Quat::from_xyzw(
                obj.orientation[0],
                obj.orientation[1],
                obj.orientation[2],
                obj.orientation[3],
            );

            // Transform the camera position into the object's local space so we
            // can compute a distance to its axis-aligned bounding box.
            let local = q.conjugate() * (cam_pos - center);
            let clamped = local.clamp(-half, half);
            let dist = (local - clamped).length();
            if dist < min_dist {
                min_dist = dist;
                // A near plane of 0.02 is sufficient; bail early once we know
                // we can't get any closer than that.
                if min_dist <= 0.04 {
                    break;
                }
            }
        }
        if !min_dist.is_finite() {
            0.1
        } else {
            (0.5 * min_dist).clamp(0.02, 0.1)
        }
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
                        // Combine intrinsic size with per-axis scale so
                        // transforms match rendered geometry.
                        let mut final_size = obj.size;
                        for i in 0..3 {
                            final_size[i] *= obj.scale[i];
                        }
                        t.size = final_size;
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
