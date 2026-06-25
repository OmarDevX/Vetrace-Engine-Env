use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use rapier3d::na::{
    Isometry3 as Isometry, Quaternion, Translation3 as Translation, UnitQuaternion,
};
use rapier3d::prelude::SharedShape;
use serde::{Deserialize, Serialize};
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GpuObject {
    /// Quaternion in (x, y, z, w) format -- must match shader layout
    pub orientation: [f32; 4],
    pub position: [f32; 3],
    pub _padding1: f32,
    pub size: [f32; 3],
    pub _padding2: f32,
    pub scale: [f32; 3],
    pub _padding2b: f32,
    pub material_index: u32,
    pub radius: f32,
    pub is_cube: u32,
    pub is_glass: u32,
    pub is_mesh: u32,
    pub triangle_start_idx: u32,
    pub triangle_count: u32,
    pub tri_bvh_start: u32,
    pub tri_bvh_count: u32,
    pub is_shaded: u32,
    pub casts_raster_shadow: u32,
    pub casts_raytraced_shadow: u32,
    pub shadow_importance: f32,
    pub max_shadow_distance: f32,
    pub scene_flags: u32,
    pub gi_flags: u32,
    pub _gi_pad0: u32,
    pub _gi_pad1: u32,
}
impl Default for GpuObject {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            _padding1: 0.0,
            size: [1.0; 3],
            _padding2: 0.0,
            scale: [1.0; 3],
            _padding2b: 0.0,
            material_index: 0,
            radius: 1.0,
            is_cube: 0,
            is_glass: 0,
            is_mesh: 0,
            triangle_start_idx: 0,
            triangle_count: 0,
            tri_bvh_start: 0,
            tri_bvh_count: 0,
            is_shaded: 1,
            casts_raster_shadow: 1,
            casts_raytraced_shadow: 0,
            shadow_importance: 0.0,
            max_shadow_distance: 100.0,
            scene_flags: SCENE_FLAG_STATIC_GEOMETRY,
            gi_flags: 0,
            _gi_pad0: 0,
            _gi_pad1: 0,
            orientation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GpuTriangle {
    pub v0: [f32; 3],
    pub _pad0: f32,
    pub e1: [f32; 3],
    pub _pad1: f32,
    pub e2: [f32; 3],
    pub _pad2: f32,
    pub n0: [f32; 3],
    pub _pad3: f32,
    pub n1: [f32; 3],
    pub _pad4: f32,
    pub n2: [f32; 3],
    pub _pad5: f32,
    pub uv0: [f32; 2],
    pub duv1: [f32; 2],
    pub duv2: [f32; 2],
    pub material_index: u32,
    pub _pad6: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GpuMaterial {
    pub base_color_factor: [f32; 4],
    pub emissive_factor: [f32; 3],
    pub emissive_strength: f32,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub ior: f32,
    pub base_color_tex: u32,
    /// Specular F0 value (vec3) aligned to 16 bytes by the following field
    pub f0: [f32; 3],
    /// Flag indicating whether this material has an associated custom shader
    pub has_custom_material: u32,
    /// Index into the `custom_materials` storage buffer
    pub custom_material_id: u32,
    /// Extra padding to keep struct size 96 bytes, matching WGSL layout.
    /// _pad2[0] mirrors MATERIAL_TAG_* fallback hints from PbrMaterial.
    pub _pad2: [u32; 7],
}

impl Default for GpuMaterial {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            emissive_factor: [0.0, 0.0, 0.0],
            emissive_strength: 0.0,
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            ior: 1.5,
            base_color_tex: 0,
            f0: [0.0, 0.0, 0.0],
            has_custom_material: 0,
            custom_material_id: 0,
            _pad2: [0; 7],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GpuCustomMaterial {
    pub color_tint: [f32; 4],
    pub base_props: [f32; 4], // roughness, metallic, noise_scale, emission_strength
    pub custom_floats: [f32; 4], // custom_float_1..4
    pub transparency_params: [f32; 4], // transparency, transmission, transmission_roughness, refraction_ior
    pub subsurface_params: [f32; 4],   // subsurface_strength, subsurface_radius.rgb
    pub coat_aniso: [f32; 4], // clearcoat_strength, clearcoat_roughness, anisotropy, anisotropy_rotation
    pub sheen_params: [f32; 4], // sheen_strength, sheen_tint.rgb
    pub normal_disp: [f32; 4], // normal_strength, displacement_strength, unused, unused
    pub texture_index: u32,
    pub output_flags: u32,
    pub material_flags: u32,
    pub _pad: u32,
}

impl Default for GpuCustomMaterial {
    fn default() -> Self {
        Self {
            color_tint: [1.0, 1.0, 1.0, 1.0],
            base_props: [0.5, 0.0, 1.0, 0.0],
            custom_floats: [0.0, 0.0, 0.0, 0.0],
            transparency_params: [0.0, 0.0, 0.0, 1.5],
            subsurface_params: [0.0, 0.0, 0.0, 0.0],
            coat_aniso: [0.0, 0.0, 0.0, 0.0],
            sheen_params: [0.0, 0.0, 0.0, 0.0],
            normal_disp: [0.0, 0.0, 0.0, 0.0],
            texture_index: 0,
            output_flags: 0,
            material_flags: 0,
            _pad: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, PartialEq)]
pub struct GpuAtmosphere {
    pub center_radius: [f32; 4],
    pub atmo_g_height: [f32; 4],
    pub ray_beta: [f32; 4],
    pub mie_beta: [f32; 4],
    pub ambient_beta: [f32; 4],
    pub absorption_beta: [f32; 4],
    pub absorb_params: [f32; 4],
    pub ozone_params: [f32; 4],
    pub absorption_layer_params: [f32; 4],
    pub multi_scatter_params: [f32; 4],
}

impl Default for GpuAtmosphere {
    fn default() -> Self {
        Self {
            center_radius: [0.0; 4],
            atmo_g_height: [0.0; 4],
            ray_beta: [0.0; 4],
            mie_beta: [0.0; 4],
            ambient_beta: [0.0; 4],
            absorption_beta: [0.0; 4],
            absorb_params: [0.0; 4],
            ozone_params: [0.0; 4],
            absorption_layer_params: [0.0; 4],
            multi_scatter_params: [0.0; 4],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, PartialEq)]
pub struct GpuVolumetricCloud {
    pub center_base_thickness: [f32; 4],
    pub coverage_density_noise_phase: [f32; 4],
    pub wind_steps: [f32; 4],
    pub light_padding: [f32; 4],
    pub multi_scatter: [f32; 4],
    pub shape_params: [f32; 4],
    pub weather_params: [f32; 4],
    pub detail_params: [f32; 4],
    pub lighting_params0: [f32; 4],
    pub lighting_params1: [f32; 4],
}

impl Default for GpuVolumetricCloud {
    fn default() -> Self {
        Self {
            center_base_thickness: [0.0; 4],
            coverage_density_noise_phase: [0.0; 4],
            wind_steps: [0.0; 4],
            light_padding: [0.0; 4],
            multi_scatter: [0.0; 4],
            shape_params: [0.0; 4],
            weather_params: [0.0; 4],
            detail_params: [0.0; 4],
            lighting_params0: [0.0; 4],
            lighting_params1: [0.0; 4],
        }
    }
}

/// Maximum number of volumetric cloud volumes supported in the scene and shader.
pub const MAX_VOLUMETRIC_CLOUDS: usize = 8;

/// Maximum number of atmospheres supported in the scene and shader.
pub const MAX_ATMOSPHERES: usize = 8;

pub const SCENE_FLAG_STATIC_GEOMETRY: u32 = 1 << 0;
pub const SCENE_FLAG_DYNAMIC_GEOMETRY: u32 = 1 << 1;
pub const SCENE_FLAG_STATIC_LIGHT: u32 = 1 << 2;
pub const SCENE_FLAG_DYNAMIC_LIGHT: u32 = 1 << 3;
pub const SCENE_FLAG_EMISSIVE_STATIC_SURFACE: u32 = 1 << 4;

#[derive(Clone, Debug, Copy, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub acceleration: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub roughness: f32,
    pub emission: f32,
    pub is_static: bool,
    pub angular_velocity: [f32; 3], // Angular velocity in radians per second
    pub angular_acceleration: [f32; 3], // Angular acceleration in radians per second^2
    /// Quaternion representing rotation in (x, y, z, w) format
    pub orientation: [f32; 4],
    pub mass: f32,
    pub is_cube: bool,
    pub size: [f32; 3],
    pub scale: [f32; 3],
    pub material_index: u32,
    pub is_glass: bool,
    pub specular_f0: [f32; 3],
    pub ior: f32,
    pub is_mesh: bool,
    pub triangle_start_idx: usize,
    pub triangle_count: usize,
    pub tri_bvh_start: usize,
    pub tri_bvh_count: usize,
    pub is_shaded: bool,
    pub casts_raster_shadow: bool,
    pub casts_raytraced_shadow: bool,
    pub shadow_importance: f32,
    pub max_shadow_distance: f32,
    pub scene_flags: u32,
    pub gi_flags: u32,
}

impl Object {
    pub fn new(
        position: [f32; 3],
        radius: f32,
        color: [f32; 3],
        roughness: f32,
        emission: f32,
        is_static: bool,
    ) -> Self {
        Object {
            position,
            velocity: [0.0; 3],
            acceleration: [0.0; 3],
            radius,
            color,
            roughness,
            emission,
            is_static,
            angular_velocity: [0.0; 3],
            angular_acceleration: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion (x, y, z, w)
            mass: 1.0,
            is_cube: true,
            size: [1.0; 3],
            scale: [1.0; 3],
            material_index: 0,
            is_glass: false,
            specular_f0: [0.0; 3],
            ior: 1.5,
            is_mesh: false,
            triangle_start_idx: 0,
            triangle_count: 0,
            tri_bvh_start: 0,
            tri_bvh_count: 0,
            is_shaded: true,
            casts_raster_shadow: true,
            casts_raytraced_shadow: false,
            shadow_importance: 0.0,
            max_shadow_distance: 100.0,
            scene_flags: SCENE_FLAG_STATIC_GEOMETRY,
            gi_flags: 0,
        }
    }
    pub fn to_gpu(&self) -> GpuObject {
        GpuObject {
            position: self.position,
            _padding1: 0.0,
            size: self.size,
            _padding2: 0.0,
            scale: self.scale,
            _padding2b: 0.0,
            material_index: self.material_index,
            radius: self.radius,
            is_cube: self.is_cube as u32,
            is_glass: self.is_glass as u32,
            is_mesh: self.is_mesh as u32,
            triangle_start_idx: self.triangle_start_idx as u32,
            triangle_count: self.triangle_count as u32,
            tri_bvh_start: self.tri_bvh_start as u32,
            tri_bvh_count: self.tri_bvh_count as u32,
            is_shaded: self.is_shaded as u32,
            casts_raster_shadow: self.casts_raster_shadow as u32,
            casts_raytraced_shadow: self.casts_raytraced_shadow as u32,
            shadow_importance: self.shadow_importance,
            max_shadow_distance: self.max_shadow_distance,
            scene_flags: self.scene_flags
                | if self.is_static {
                    SCENE_FLAG_STATIC_GEOMETRY
                } else {
                    SCENE_FLAG_DYNAMIC_GEOMETRY
                },
            gi_flags: self.gi_flags
                | if self.is_static && self.emission > 0.0 {
                    SCENE_FLAG_EMISSIVE_STATIC_SURFACE
                } else {
                    0
                },
            _gi_pad0: 0,
            _gi_pad1: 0,
            orientation: self.orientation,
        }
    }
    pub fn process_physics(&mut self, delta_time: f32, spheres: &mut [Object]) {
        let mut apply_gravity = true;

        if !self.is_static {
            for other in spheres.iter_mut() {
                if self.position != other.position && self != other {
                    let iso1 = self.iso();
                    let iso2 = other.iso();
                    let s1 = self.shape();
                    let s2 = other.shape();
                    if rapier3d::parry::query::intersection_test(&iso1, &*s1, &iso2, &*s2)
                        .unwrap_or(false)
                    {
                        self.resolve_collision(other);
                        apply_gravity = false;
                    }
                }
            }
            if apply_gravity {
                self.velocity[0] += self.acceleration[0] * delta_time;
                self.velocity[1] += self.acceleration[1] * delta_time;
                self.velocity[2] += self.acceleration[2] * delta_time;

                // Limit velocity to prevent excessive speed
                let max_speed: f32 = 5.0; // Adjust as needed
                let speed_squared =
                    self.velocity[0].powi(2) + self.velocity[1].powi(2) + self.velocity[2].powi(2);
                if speed_squared > max_speed.powi(2) {
                    let speed = speed_squared.sqrt();
                    self.velocity[0] = (self.velocity[0] / speed) * max_speed;
                    self.velocity[1] = (self.velocity[1] / speed) * max_speed;
                    self.velocity[2] = (self.velocity[2] / speed) * max_speed;
                }

                // Update position using Verlet integration
                self.position[0] += self.velocity[0] * delta_time;
                self.position[1] += self.velocity[1] * delta_time;
                self.position[2] += self.velocity[2] * delta_time;
            }
            // Update angular velocity and orientation
            self.angular_velocity[0] += self.angular_acceleration[0] * delta_time;
            self.angular_velocity[1] += self.angular_acceleration[1] * delta_time;
            self.angular_velocity[2] += self.angular_acceleration[2] * delta_time;

            // Limit angular velocity to prevent excessive rotation
            let max_angular_speed: f32 = 10.0; // Adjust as needed
            let angular_speed_squared = self.angular_velocity[0].powi(2)
                + self.angular_velocity[1].powi(2)
                + self.angular_velocity[2].powi(2);
            if angular_speed_squared > max_angular_speed.powi(2) {
                let angular_speed = angular_speed_squared.sqrt();
                self.angular_velocity[0] =
                    self.angular_velocity[0] / angular_speed * max_angular_speed;
                self.angular_velocity[1] =
                    self.angular_velocity[1] / angular_speed * max_angular_speed;
                self.angular_velocity[2] =
                    self.angular_velocity[2] / angular_speed * max_angular_speed;
            }
            let t_vec = Vec3::new(self.velocity[0], self.velocity[1], self.velocity[2]);
            if t_vec.length_squared() < 0.5 {
                self.velocity = [0.0; 3];
            }

            self.update_orientation(delta_time);
        }

        // self.check_collision(spheres);
    }

    fn update_orientation(&mut self, delta_time: f32) {
        let angle = (self.angular_velocity[0].powi(2)
            + self.angular_velocity[1].powi(2)
            + self.angular_velocity[2].powi(2))
        .sqrt()
            * delta_time;
        if angle != 0.0 {
            let axis = [
                self.angular_velocity[0] / angle,
                self.angular_velocity[1] / angle,
                self.angular_velocity[2] / angle,
            ];
            let half_angle = angle * 0.5;
            let sin_half_angle = half_angle.sin();
            // Quaternion representing the rotation in (x, y, z, w) format
            let delta_orientation = [
                axis[0] * sin_half_angle,
                axis[1] * sin_half_angle,
                axis[2] * sin_half_angle,
                half_angle.cos(),
            ];
            self.orientation = Object::quaternion_multiply(self.orientation, delta_orientation);
            let n = (self.orientation[0] * self.orientation[0]
                + self.orientation[1] * self.orientation[1]
                + self.orientation[2] * self.orientation[2]
                + self.orientation[3] * self.orientation[3])
                .sqrt();
            self.orientation[0] /= n;
            self.orientation[1] /= n;
            self.orientation[2] /= n;
            self.orientation[3] /= n;
        }
    }

    fn quaternion_multiply(q1: [f32; 4], q2: [f32; 4]) -> [f32; 4] {
        [
            q1[3] * q2[0] + q1[0] * q2[3] + q1[1] * q2[2] - q1[2] * q2[1],
            q1[3] * q2[1] - q1[0] * q2[2] + q1[1] * q2[3] + q1[2] * q2[0],
            q1[3] * q2[2] + q1[0] * q2[1] - q1[1] * q2[0] + q1[2] * q2[3],
            q1[3] * q2[3] - q1[0] * q2[0] - q1[1] * q2[1] - q1[2] * q2[2],
        ]
    }

    fn iso(&self) -> Isometry<f32> {
        let q = UnitQuaternion::from_quaternion(Quaternion::new(
            self.orientation[3],
            self.orientation[0],
            self.orientation[1],
            self.orientation[2],
        ));
        Isometry::from_parts(
            Translation::new(self.position[0], self.position[1], self.position[2]),
            q,
        )
    }

    fn shape(&self) -> SharedShape {
        if self.is_cube {
            SharedShape::cuboid(
                self.size[0] * self.scale[0] * 0.5,
                self.size[1] * self.scale[1] * 0.5,
                self.size[2] * self.scale[2] * 0.5,
            )
        } else {
            let s = self.scale[0].max(self.scale[1]).max(self.scale[2]);
            SharedShape::ball(self.radius * s)
        }
    }

    // fn check_collision(&mut self, spheres: &mut [Sphere]) {
    //     for other in spheres.iter_mut() {
    //         if self.position != other.position&&self!=other {
    //             let distance = Sphere::distance_between_spheres(self, other);
    //             if distance < self.radius + other.radius {
    //                 self.resolve_collision(other);
    //             }
    //         }
    //     }
    // }

    fn resolve_collision(&mut self, other: &mut Object) {
        if self == other {
            return;
        }

        let iso1 = self.iso();
        let iso2 = other.iso();
        let s1 = self.shape();
        let s2 = other.shape();

        if let Ok(Some(contact)) = rapier3d::parry::query::contact(&iso1, &*s1, &iso2, &*s2, 0.0) {
            let penetration = contact.normal1.into_inner() * -contact.dist;
            if self.is_static && other.is_static {
                // Two static objects should not move
            } else if !self.is_static && other.is_static {
                self.position[0] += penetration.x;
                self.position[1] += penetration.y;
                self.position[2] += penetration.z;
            } else if self.is_static && !other.is_static {
                other.position[0] -= penetration.x;
                other.position[1] -= penetration.y;
                other.position[2] -= penetration.z;
            } else {
                // Both objects are non-static; no positional correction
            }
        }
    }

    pub fn update(&mut self, delta_time: f32, spheres: &mut [Object]) {
        let fixed_delta_time = 0.016; // Example: Fixed time step of 0.016 seconds (60 FPS)
        let mut time_accumulator = delta_time;

        while time_accumulator >= fixed_delta_time {
            self.process_physics(fixed_delta_time, spheres);
            time_accumulator -= fixed_delta_time;
        }

        // Process remaining time (if any) with a smaller time step
        if time_accumulator > 0.0 {
            self.process_physics(time_accumulator, spheres);
        }
    }

    fn distance_between_spheres(s1: &Object, s2: &Object) -> f32 {
        let dx = s1.position[0] - s2.position[0];
        let dy = s1.position[1] - s2.position[1];
        let dz = s1.position[2] - s2.position[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
impl Default for Object {
    fn default() -> Self {
        Object {
            position: [0.0; 3],
            velocity: [0.0; 3],
            acceleration: [0.0; 3],
            radius: 1.0,
            // Default to mid-gray in 0-1 range
            color: [0.47; 3],
            roughness: 1.0,
            emission: 0.0,
            is_static: true,
            angular_velocity: [0.0; 3],
            angular_acceleration: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
            mass: 1.0,
            is_cube: true,
            size: [1.0; 3],
            scale: [1.0; 3],
            material_index: 0,
            is_glass: false,
            specular_f0: [0.0; 3],
            ior: 1.5,
            is_mesh: false,
            triangle_start_idx: 0,
            triangle_count: 0,
            tri_bvh_start: 0,
            tri_bvh_count: 0,
            is_shaded: true,
            casts_raster_shadow: true,
            casts_raytraced_shadow: false,
            shadow_importance: 0.0,
            max_shadow_distance: 100.0,
            scene_flags: SCENE_FLAG_STATIC_GEOMETRY,
            gi_flags: 0,
        }
    }
}
