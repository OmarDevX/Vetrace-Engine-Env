//! Reflection lab demonstrating the complete cubemap/probe feature set.
//!
//! Run from the workspace root:
//! `cargo run -p vetrace_render --example cubemap_environment --features wgpu_window`
//!
//! Features shown in one scene:
//! - global cubemap sky and IBL
//! - baked and realtime local reflection probes
//! - box-projected parallax correction
//! - smooth overlapping-probe transitions
//! - mirror, glossy, rough-metal, and dielectric reference materials
//! - an animated/movable object, light, and RGB marker captured by the realtime probe
//! - experimental screen-space reflections layered over probe fallback
//! - SSAO, bloom, ACES tone mapping, exposure, FXAA, and soft shadows
//!
//! Controls:
//! - Mouse + W/A/S/D + E/Q: free-flight camera
//! - Shift: movement boost; wheel: change flight speed
//! - Arrow keys: move the orange reflected object; M: automatic motion
//! - R: recapture both probes
//! - P: toggle probe parallax correction
//! - 1: toggle SSR; 2: toggle bloom; 3: toggle SSAO; 4: toggle sky
//! - T: cycle tone mapper; [ / ]: lower / raise exposure
//! - Escape: quit

use glam::{Quat, Vec2, Vec3};
use vetrace_core::{Actor, App, AppBuilder, Engine, InputState, Transform};
use vetrace_render::{
    AmbientOcclusionMode, AntiAliasingMode, Bloom, Camera, CubemapAsset, CubemapHandle,
    DirectionalLight, EnvironmentCubemap, FreeFlightCameraController, Material, PointLight,
    PostProcessing, PrimitiveShape, ReflectionProbe, ReflectionProbeCaptureMode,
    ReflectionProbeCaptureRequests,
    ReflectionProbeParallaxMode, RenderAssets, RenderBundle, RenderLayers, RenderPlugin,
    RenderSettings, Renderable, ScreenSpaceReflections, ShadowFilterMode, ShadowMode, Shape,
    TextureAsset, TextureHandle, ToneMapper,
};

const MIRROR_LAYER: u32 = 1 << 1;
const MAIN_PROBE_NAME: &str = "Main Baked Reflection Probe";
const DYNAMIC_PROBE_NAME: &str = "Realtime Reflection Probe";
struct CubemapEnvironmentExample {
    camera_controller: FreeFlightCameraController,
    main_probe: Actor,
    dynamic_probe: Actor,
    moving_object: Actor,
    moving_light: Actor,
    moving_markers: [Actor; 3],
    moving_object_origin: Vec3,
    animation_time: f32,
    automatic_motion: bool,
    parallax_enabled: bool,
    ssr_enabled: bool,
    bloom_enabled: bool,
    ssao_enabled: bool,
    sky_enabled: bool,
    cool_sky: Option<CubemapHandle>,
    warm_sky: Option<CubemapHandle>,
    warm_sky_active: bool,
    sky_transition_active: bool,
}

impl Default for CubemapEnvironmentExample {
    fn default() -> Self {
        Self {
            camera_controller: FreeFlightCameraController::default()
                .with_movement_speed(4.5),
            main_probe: Actor::INVALID,
            dynamic_probe: Actor::INVALID,
            moving_object: Actor::INVALID,
            moving_light: Actor::INVALID,
            moving_markers: [Actor::INVALID; 3],
            moving_object_origin: Vec3::new(3.8, 1.0, 0.0),
            animation_time: 0.0,
            automatic_motion: true,
            parallax_enabled: true,
            ssr_enabled: true,
            bloom_enabled: true,
            ssao_enabled: true,
            sky_enabled: true,
            cool_sky: None,
            warm_sky: None,
            warm_sky_active: false,
            sky_transition_active: false,
        }
    }
}

impl App for CubemapEnvironmentExample {
    fn setup(&mut self, engine: &mut Engine) {
        print_controls();

        engine.insert_resource(Camera {
            position: Vec3::new(0.0, 2.25, 9.2),
            target: Vec3::new(0.0, 1.7, 0.0),
            near: 0.04,
            far: 100.0,
            ..Camera::default()
        });

        engine.insert_resource(PostProcessing {
            bloom: Bloom {
                enabled: true,
                threshold: 0.72,
                intensity: 0.35,
                radius: 5.0,
            },
            exposure: 1.08,
            gamma: 2.2,
            tone_mapper: ToneMapper::Aces,
            ..PostProcessing::default()
        });

        engine.insert_resource(ScreenSpaceReflections {
            enabled: true,
            intensity: 0.48,
            max_distance: 8.5,
            thickness: 0.22,
            stride: 0.16,
            max_steps: 56,
            edge_fade: 0.11,
            start_distance: 0.26,
            origin_bias: 0.04,
            distance_fade_start: 0.60,
            normal_rejection: 0.10,
            max_confidence: 0.86,
            temporal_enabled: true,
            temporal_weight: 0.16,
            history_clamp: 0.07,
            disocclusion_threshold: 0.18,
            ..ScreenSpaceReflections::default()
        });

        let sky = insert_studio_sky_cubemap(engine, "reflection lab cool studio sky", 0.0);
        let warm_sky = insert_studio_sky_cubemap(engine, "reflection lab warm studio sky", 1.0);
        self.cool_sky = Some(sky);
        self.warm_sky = Some(warm_sky);
        *engine
            .get_resource_mut::<EnvironmentCubemap>()
            .expect("RenderPlugin inserts EnvironmentCubemap") = EnvironmentCubemap {
            enabled: true,
            primary: Some(sky),
            intensity: 0.72,
            draw_sky: true,
            diffuse_ibl: true,
            specular_ibl: true,
            ..EnvironmentCubemap::default()
        };

        let floor_texture = insert_checker_texture(
            engine,
            "reflection lab checker floor",
            256,
            Vec3::new(0.055, 0.065, 0.075),
            Vec3::new(0.22, 0.24, 0.26),
            16,
        );
        let brick_texture = insert_brick_texture(engine, "reflection lab brick", 256);
        let panel_texture = insert_panel_texture(engine, "reflection lab panels", 256);

        spawn_lab_room(engine, floor_texture, brick_texture, panel_texture);
        spawn_lighting(engine);

        self.main_probe = engine
            .spawn_actor(MAIN_PROBE_NAME)
            .with(Transform {
                translation: Vec3::new(-2.3, 2.45, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .with(ReflectionProbe {
                half_extents: Vec3::new(4.7, 2.65, 4.8),
                blend_distance: 1.35,
                priority: 10,
                capture_mode: ReflectionProbeCaptureMode::Baked,
                capture_resolution: 512,
                capture_near: 0.04,
                capture_far: 40.0,
                transition_seconds: 0.45,
                capture_shadows: true,
                invalidation_mode: vetrace_render::ReflectionProbeInvalidationMode::SceneChanges,
                capture_exclude_layers: MIRROR_LAYER,
                ..ReflectionProbe::default()
            })
            .build();

        self.dynamic_probe = engine
            .spawn_actor(DYNAMIC_PROBE_NAME)
            .with(Transform {
                translation: Vec3::new(3.7, 2.35, 0.0),
                rotation: Quat::from_rotation_y(-8.0_f32.to_radians()),
                scale: Vec3::ONE,
            })
            .with(ReflectionProbe {
                half_extents: Vec3::new(3.5, 2.6, 4.7),
                blend_distance: 1.5,
                priority: 12,
                capture_mode: ReflectionProbeCaptureMode::Realtime,
                capture_resolution: 256,
                capture_near: 0.04,
                capture_far: 35.0,
                transition_seconds: 0.28,
                update_interval_seconds: 0.42,
                capture_exclude_layers: MIRROR_LAYER,
                ..ReflectionProbe::default()
            })
            .build();

        spawn_material_reference_row(engine);
        spawn_feature_objects(engine);

        self.moving_object = spawn_layered_shape(
            engine,
            "Realtime Reflected Orange Object",
            PrimitiveShape::Cube,
            self.moving_object_origin,
            Vec3::new(0.75, 1.5, 0.75),
            Material {
                base_color: Vec3::new(1.0, 0.16, 0.025),
                emissive: Vec3::new(0.55, 0.035, 0.005),
                metallic: 0.18,
                roughness: 0.22,
                ..Material::default()
            },
            u32::MAX,
        );
        self.moving_light = spawn_point_light(
            engine,
            "Realtime Moving Orange Light",
            self.moving_object_origin + Vec3::new(0.0, 0.85, 0.0),
            Vec3::new(1.0, 0.18, 0.025),
            16.0,
            4.5,
        );
        self.moving_markers = [
            spawn_marker_bar(
                engine,
                "Moving Marker X",
                self.moving_object_origin + Vec3::new(0.72, 1.05, 0.0),
                Vec3::new(1.10, 0.10, 0.10),
                Vec3::new(1.0, 0.08, 0.035),
            ),
            spawn_marker_bar(
                engine,
                "Moving Marker Y",
                self.moving_object_origin + Vec3::new(0.0, 1.72, 0.0),
                Vec3::new(0.10, 1.10, 0.10),
                Vec3::new(0.10, 1.0, 0.08),
            ),
            spawn_marker_bar(
                engine,
                "Moving Marker Z",
                self.moving_object_origin + Vec3::new(0.0, 1.05, 0.72),
                Vec3::new(0.10, 0.10, 1.10),
                Vec3::new(0.08, 0.34, 1.0),
            ),
        ];
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        self.animation_time += dt;
        self.update_free_flight_camera(engine, dt);
        self.advance_sky_transition(engine, dt);
        self.handle_feature_controls(engine, dt);
        self.update_moving_object(engine, dt);

        if engine
            .get_resource::<InputState>()
            .is_some_and(|input| input.quit_requested() || input.was_key_pressed("Escape"))
        {
            engine.stop();
        }
    }
}

impl CubemapEnvironmentExample {
    fn update_free_flight_camera(&mut self, engine: &mut Engine, dt: f32) {
        let Some(input) = engine.get_resource::<InputState>().cloned() else {
            return;
        };
        if let Some(camera) = engine.get_resource_mut::<Camera>() {
            self.camera_controller.update(&input, camera, dt);
        }
    }

    fn advance_sky_transition(&mut self, engine: &mut Engine, dt: f32) {
        if !self.sky_transition_active {
            return;
        }
        if let Some(environment) = engine.get_resource_mut::<EnvironmentCubemap>() {
            if environment.advance_transition(dt, 1.5) {
                self.sky_transition_active = false;
                self.warm_sky_active = !self.warm_sky_active;
                println!("Global cubemap transition complete");
            }
        }
    }

    fn handle_feature_controls(&mut self, engine: &mut Engine, dt: f32) {
        let Some(input) = engine.get_resource::<InputState>().cloned() else {
            return;
        };

        if input.was_key_pressed("Digit1") {
            self.ssr_enabled = !self.ssr_enabled;
            if let Some(ssr) = engine.get_resource_mut::<ScreenSpaceReflections>() {
                ssr.enabled = self.ssr_enabled;
            }
            println!("SSR: {}", on_off(self.ssr_enabled));
        }
        if input.was_key_pressed("Digit2") {
            self.bloom_enabled = !self.bloom_enabled;
            if let Some(post) = engine.get_resource_mut::<PostProcessing>() {
                post.bloom.enabled = self.bloom_enabled;
            }
            println!("Bloom: {}", on_off(self.bloom_enabled));
        }
        if input.was_key_pressed("Digit3") {
            self.ssao_enabled = !self.ssao_enabled;
            if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
                settings.ambient_occlusion_mode = if self.ssao_enabled {
                    AmbientOcclusionMode::Ssao
                } else {
                    AmbientOcclusionMode::Off
                };
            }
            println!("SSAO: {}", on_off(self.ssao_enabled));
        }
        if input.was_key_pressed("Digit4") {
            self.sky_enabled = !self.sky_enabled;
            if let Some(environment) = engine.get_resource_mut::<EnvironmentCubemap>() {
                environment.draw_sky = self.sky_enabled;
            }
            println!("Visible sky: {}", on_off(self.sky_enabled));
        }
        if input.was_key_pressed("G") && !self.sky_transition_active {
            let target = if self.warm_sky_active { self.cool_sky } else { self.warm_sky };
            if let Some(target) = target {
                if let Some(environment) = engine.get_resource_mut::<EnvironmentCubemap>() {
                    environment.begin_transition(target);
                    self.sky_transition_active = true;
                    println!("Started global cubemap crossfade");
                }
            }
        }
        if input.was_key_pressed("M") {
            self.automatic_motion = !self.automatic_motion;
            println!("Automatic reflected-object motion: {}", on_off(self.automatic_motion));
        }
        if input.was_key_pressed("P") {
            self.parallax_enabled = !self.parallax_enabled;
            let mode = if self.parallax_enabled {
                ReflectionProbeParallaxMode::BoxProjection
            } else {
                ReflectionProbeParallaxMode::Disabled
            };
            for actor in [self.main_probe, self.dynamic_probe] {
                if actor.is_alive(engine) {
                    if let Some(probe) = engine.raw_world_mut().get_mut::<ReflectionProbe>(actor.entity()) {
                        probe.parallax_mode = mode;
                    }
                }
            }
            println!("Probe box projection: {}", on_off(self.parallax_enabled));
        }
        if input.was_key_pressed("R") {
            let main_probe = self.main_probe.is_alive(engine).then_some(self.main_probe.entity());
            let dynamic_probe = self.dynamic_probe.is_alive(engine).then_some(self.dynamic_probe.entity());
            if let Some(requests) = engine.get_resource_mut::<ReflectionProbeCaptureRequests>() {
                if let Some(entity) = main_probe { requests.request(entity); }
                if let Some(entity) = dynamic_probe { requests.request(entity); }
            }
            println!("Requested fresh captures for both reflection probes");
        }
        if input.was_key_pressed("T") {
            if let Some(post) = engine.get_resource_mut::<PostProcessing>() {
                post.tone_mapper = post.tone_mapper.next();
                println!("Tone mapper: {:?}", post.tone_mapper);
            }
        }
        if input.was_key_pressed("BracketLeft") {
            if let Some(post) = engine.get_resource_mut::<PostProcessing>() {
                post.exposure = (post.exposure - 0.08).max(0.25);
                println!("Exposure: {:.2}", post.exposure);
            }
        }
        if input.was_key_pressed("BracketRight") {
            if let Some(post) = engine.get_resource_mut::<PostProcessing>() {
                post.exposure = (post.exposure + 0.08).min(3.0);
                println!("Exposure: {:.2}", post.exposure);
            }
        }

        if !self.automatic_motion && self.moving_object.is_alive(engine) {
            let mut movement = Vec3::ZERO;
            if input.is_key_down("ArrowLeft") { movement.x -= 1.0; }
            if input.is_key_down("ArrowRight") { movement.x += 1.0; }
            if input.is_key_down("ArrowUp") { movement.z -= 1.0; }
            if input.is_key_down("ArrowDown") { movement.z += 1.0; }
            if movement.length_squared() > 0.0 {
                let mut new_position = None;
                if let Some(transform) = self.moving_object.transform_mut(engine) {
                    transform.translation += movement.normalize() * 2.4 * dt;
                    transform.translation.x = transform.translation.x.clamp(1.4, 6.0);
                    transform.translation.z = transform.translation.z.clamp(-3.6, 3.6);
                    new_position = Some(transform.translation);
                }
                if let Some(position) = new_position {
                    self.update_moving_group(engine, position);
                }
            }
        }
    }

    fn update_moving_group(&self, engine: &mut Engine, position: Vec3) {
        if let Some(light_transform) = self.moving_light.transform_mut(engine) {
            light_transform.translation = position + Vec3::new(0.0, 0.85, 0.0);
        }
        let offsets = [
            Vec3::new(0.72, 1.05, 0.0),
            Vec3::new(0.0, 1.72, 0.0),
            Vec3::new(0.0, 1.05, 0.72),
        ];
        for (marker, offset) in self.moving_markers.into_iter().zip(offsets) {
            if let Some(transform) = marker.transform_mut(engine) {
                transform.translation = position + offset;
            }
        }
    }

    fn update_moving_object(&mut self, engine: &mut Engine, _dt: f32) {
        if !self.automatic_motion || !self.moving_object.is_alive(engine) {
            return;
        }
        let mut new_position = None;
        if let Some(transform) = self.moving_object.transform_mut(engine) {
            let t = self.animation_time;
            transform.translation = self.moving_object_origin
                + Vec3::new((t * 0.72).sin() * 1.45, (t * 1.3).sin() * 0.28, (t * 0.51).cos() * 1.55);
            transform.rotation = Quat::from_rotation_y(t * 0.75) * Quat::from_rotation_x(t * 0.31);
            new_position = Some(transform.translation);
        }
        if let Some(position) = new_position {
            self.update_moving_group(engine, position);
        }
    }
}

fn print_controls() {
    println!("Vetrace reflection lab controls:");
    println!("  Mouse + W/A/S/D + E/Q   free-flight camera");
    println!("  Shift / mouse wheel     boost / flight speed");
    println!("  Arrow keys / M          move object / toggle automatic motion");
    println!("  G                       crossfade the global cubemap");
    println!("  R                       recapture both probes");
    println!("  P                       toggle box-projected parallax");
    println!("  1 / 2 / 3 / 4           SSR / bloom / SSAO / visible sky");
    println!("  T / [ / ]               tone mapper / exposure down / up");
    println!("  Escape                  quit");
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "ON" } else { "OFF" }
}

fn spawn_lab_room(
    engine: &mut Engine,
    floor_texture: TextureHandle,
    brick_texture: TextureHandle,
    panel_texture: TextureHandle,
) {
    spawn_box(
        engine,
        "Glossy Checker Floor",
        Vec3::new(0.0, -0.15, 0.0),
        Vec3::new(14.0, 0.3, 10.0),
        Material {
            base_color: Vec3::splat(0.78),
            base_color_texture: Some(floor_texture),
            uv_scale: Vec2::splat(1.8),
            metallic: 0.18,
            roughness: 0.24,
            ..Material::default()
        },
    );
    spawn_box(
        engine,
        "Brick Back Wall",
        Vec3::new(0.0, 2.55, -5.0),
        Vec3::new(14.0, 5.4, 0.28),
        Material {
            base_color: Vec3::new(0.48, 0.53, 0.58),
            base_color_texture: Some(brick_texture),
            uv_scale: Vec2::splat(1.25),
            roughness: 0.76,
            ..Material::default()
        },
    );
    spawn_box(
        engine,
        "Left Brick Wall",
        Vec3::new(-7.0, 2.55, 0.0),
        Vec3::new(0.28, 5.4, 10.0),
        Material {
            base_color: Vec3::new(0.50, 0.58, 0.50),
            base_color_texture: Some(brick_texture),
            uv_scale: Vec2::splat(1.2),
            roughness: 0.78,
            ..Material::default()
        },
    );
    spawn_box(
        engine,
        "Right Panel Wall",
        Vec3::new(7.0, 2.55, 0.0),
        Vec3::new(0.28, 5.4, 10.0),
        Material {
            base_color: Vec3::new(0.30, 0.34, 0.42),
            base_color_texture: Some(panel_texture),
            uv_scale: Vec2::splat(1.0),
            metallic: 0.22,
            roughness: 0.54,
            ..Material::default()
        },
    );
    spawn_box(
        engine,
        "Dark Ceiling",
        Vec3::new(0.0, 5.15, 0.0),
        Vec3::new(14.0, 0.25, 10.0),
        Material {
            base_color: Vec3::new(0.065, 0.075, 0.09),
            metallic: 0.15,
            roughness: 0.64,
            ..Material::default()
        },
    );

    // Divider pieces define a visible probe transition doorway without closing
    // the two capture zones off from one another.
    let divider_material = Material {
        base_color: Vec3::new(0.08, 0.10, 0.13),
        metallic: 0.62,
        roughness: 0.30,
        ..Material::default()
    };
    spawn_box(engine, "Probe Divider Left", Vec3::new(1.0, 2.55, -3.85), Vec3::new(0.45, 5.1, 2.3), divider_material.clone());
    spawn_box(engine, "Probe Divider Right", Vec3::new(1.0, 2.55, 3.85), Vec3::new(0.45, 5.1, 2.3), divider_material.clone());
    spawn_box(engine, "Probe Divider Header", Vec3::new(1.0, 4.55, 0.0), Vec3::new(0.45, 1.1, 5.4), divider_material);

    spawn_emissive_panel(
        engine,
        "Cool Light Panel",
        Vec3::new(-6.78, 2.9, -0.8),
        Vec3::new(0.12, 2.7, 2.0),
        Vec3::new(0.45, 0.95, 1.0),
        5.0,
    );
    spawn_emissive_panel(
        engine,
        "Warm Light Panel",
        Vec3::new(6.78, 2.65, 1.2),
        Vec3::new(0.12, 2.4, 2.2),
        Vec3::new(1.0, 0.40, 0.12),
        4.5,
    );
}

fn spawn_lighting(engine: &mut Engine) {
    engine
        .spawn_actor("Soft Key Directional Light")
        .with(DirectionalLight {
            direction: Vec3::new(-0.35, -1.0, -0.22).normalize(),
            color: Vec3::new(0.78, 0.88, 1.0),
            intensity: 1.3,
            shadow_mode: ShadowMode::Soft,
        })
        .build();

    spawn_point_light(engine, "Cool Panel Light", Vec3::new(-4.9, 2.9, -0.7), Vec3::new(0.25, 0.82, 1.0), 42.0, 8.0);
    spawn_point_light(engine, "Warm Panel Light", Vec3::new(5.0, 2.65, 1.1), Vec3::new(1.0, 0.28, 0.07), 38.0, 7.5);
    spawn_point_light(engine, "Ceiling Fill", Vec3::new(0.4, 4.3, -1.5), Vec3::new(0.55, 0.62, 0.78), 18.0, 9.0);
}

fn spawn_material_reference_row(engine: &mut Engine) {
    let roughness_values = [0.025, 0.10, 0.24, 0.46, 0.78];
    for (index, roughness) in roughness_values.into_iter().enumerate() {
        spawn_layered_shape(
            engine,
            &format!("Metal Roughness {:.3}", roughness),
            PrimitiveShape::Sphere,
            Vec3::new(-5.25 + index as f32 * 1.18, 0.63, -2.35),
            Vec3::splat(0.88),
            Material {
                base_color: Vec3::new(0.86, 0.90, 0.96),
                metallic: 1.0,
                roughness,
                double_sided: false,
                ..Material::default()
            },
            MIRROR_LAYER,
        );
    }
}

fn spawn_feature_objects(engine: &mut Engine) {
    spawn_layered_shape(
        engine,
        "Hero Mirror Sphere",
        PrimitiveShape::Sphere,
        Vec3::new(-2.35, 1.55, 0.15),
        Vec3::splat(2.65),
        Material {
            base_color: Vec3::splat(0.96),
            metallic: 1.0,
            roughness: 0.012,
            double_sided: false,
            ..Material::default()
        },
        MIRROR_LAYER,
    );
    spawn_layered_shape(
        engine,
        "Glossy Dielectric Sphere",
        PrimitiveShape::Sphere,
        Vec3::new(0.10, 1.08, 1.15),
        Vec3::splat(1.55),
        Material {
            base_color: Vec3::new(0.045, 0.13, 0.22),
            metallic: 0.0,
            roughness: 0.07,
            specular_f0: Vec3::splat(0.06),
            double_sided: false,
            ..Material::default()
        },
        MIRROR_LAYER,
    );
    spawn_layered_shape(
        engine,
        "Brushed Metal Cube",
        PrimitiveShape::Cube,
        Vec3::new(-4.55, 1.05, 1.25),
        Vec3::new(1.45, 2.1, 1.45),
        Material {
            base_color: Vec3::new(0.60, 0.64, 0.70),
            metallic: 0.94,
            roughness: 0.32,
            ..Material::default()
        },
        MIRROR_LAYER,
    );
    spawn_layered_shape(
        engine,
        "Realtime Bay Chrome Sphere",
        PrimitiveShape::Sphere,
        Vec3::new(4.55, 1.25, -1.35),
        Vec3::splat(1.9),
        Material {
            base_color: Vec3::splat(0.94),
            metallic: 1.0,
            roughness: 0.025,
            double_sided: false,
            ..Material::default()
        },
        MIRROR_LAYER,
    );
}

fn spawn_emissive_panel(
    engine: &mut Engine,
    name: &str,
    position: Vec3,
    size: Vec3,
    color: Vec3,
    emission: f32,
) {
    spawn_box(
        engine,
        name,
        position,
        size,
        Material {
            base_color: color,
            emissive: color * emission,
            roughness: 0.18,
            ..Material::default()
        },
    );
}

fn spawn_marker_bar(
    engine: &mut Engine,
    name: &str,
    position: Vec3,
    size: Vec3,
    color: Vec3,
) -> Actor {
    spawn_box(
        engine,
        name,
        position,
        size,
        Material {
            base_color: color,
            emissive: color * 3.5,
            metallic: 0.1,
            roughness: 0.16,
            ..Material::default()
        },
    )
}

fn spawn_point_light(
    engine: &mut Engine,
    name: &str,
    position: Vec3,
    color: Vec3,
    intensity: f32,
    range: f32,
) -> Actor {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            ..Transform::default()
        })
        .with(PointLight {
            color,
            intensity,
            range: Some(range),
            shadow_mode: ShadowMode::None,
        })
        .build()
}

fn insert_studio_sky_cubemap(
    engine: &mut Engine,
    name: &str,
    warmth: f32,
) -> CubemapHandle {
    const SIZE: u32 = 128;
    let mut faces: [Vec<u8>; CubemapAsset::FACE_COUNT] = std::array::from_fn(|_| {
        Vec::with_capacity((SIZE * SIZE * 4) as usize)
    });
    for (face_index, face) in faces.iter_mut().enumerate() {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let u = x as f32 / (SIZE - 1) as f32;
                let v = y as f32 / (SIZE - 1) as f32;
                let vertical = 1.0 - v;
                let horizon = (1.0 - (vertical - 0.44).abs() * 2.0).clamp(0.0, 1.0);
                let face_tint = match face_index {
                    0 => Vec3::new(0.08, 0.10, 0.16),
                    1 => Vec3::new(0.10, 0.07, 0.14),
                    2 => Vec3::new(0.20, 0.28, 0.42),
                    3 => Vec3::new(0.018, 0.022, 0.030),
                    4 => Vec3::new(0.07, 0.12, 0.20),
                    _ => Vec3::new(0.13, 0.08, 0.15),
                };
                let top = Vec3::new(0.08, 0.13, 0.24);
                let bottom = Vec3::new(0.012, 0.015, 0.023);
                let warm_horizon = Vec3::new(0.22, 0.095, 0.055) * horizon * (0.45 + warmth * 0.75);
                let warm_shift = Vec3::new(0.12, 0.035, -0.015) * warmth;
                let subtle_band = (u * std::f32::consts::TAU + face_index as f32).sin() * 0.008;
                let color = bottom.lerp(top, vertical) + face_tint + warm_horizon + warm_shift + Vec3::splat(subtle_band);
                push_rgba8(face, color, 1.0);
            }
        }
    }
    let cubemap = CubemapAsset::from_faces_rgba8(name, SIZE, faces)
        .expect("procedural sky cubemap faces are valid");
    engine
        .get_resource_mut::<RenderAssets>()
        .expect("RenderPlugin inserts RenderAssets")
        .insert_cubemap(cubemap)
}

fn insert_checker_texture(
    engine: &mut Engine,
    name: &str,
    size: u32,
    a: Vec3,
    b: Vec3,
    cells: u32,
) -> TextureHandle {
    let mut rgba8 = Vec::with_capacity((size * size * 4) as usize);
    let cell_size = (size / cells.max(1)).max(1);
    for y in 0..size {
        for x in 0..size {
            let use_a = ((x / cell_size) + (y / cell_size)) % 2 == 0;
            push_rgba8(&mut rgba8, if use_a { a } else { b }, 1.0);
        }
    }
    insert_texture(engine, name, size, size, rgba8)
}

fn insert_brick_texture(engine: &mut Engine, name: &str, size: u32) -> TextureHandle {
    let mut rgba8 = Vec::with_capacity((size * size * 4) as usize);
    let brick_w = 48_u32;
    let brick_h = 24_u32;
    for y in 0..size {
        for x in 0..size {
            let row = y / brick_h;
            let shifted_x = x + if row % 2 == 0 { 0 } else { brick_w / 2 };
            let mortar = y % brick_h < 2 || shifted_x % brick_w < 2;
            let noise = (((x * 17 + y * 31) % 23) as f32 / 22.0 - 0.5) * 0.035;
            let brick = Vec3::new(0.34 + noise, 0.30 + noise, 0.28 + noise);
            let color = if mortar { Vec3::new(0.09, 0.10, 0.105) } else { brick };
            push_rgba8(&mut rgba8, color, 1.0);
        }
    }
    insert_texture(engine, name, size, size, rgba8)
}

fn insert_panel_texture(engine: &mut Engine, name: &str, size: u32) -> TextureHandle {
    let mut rgba8 = Vec::with_capacity((size * size * 4) as usize);
    let cell = 64_u32;
    for y in 0..size {
        for x in 0..size {
            let seam = x % cell < 3 || y % cell < 3;
            let dot = (x % cell).abs_diff(cell / 2) < 2 && (y % cell).abs_diff(cell / 2) < 2;
            let color = if seam {
                Vec3::new(0.025, 0.03, 0.04)
            } else if dot {
                Vec3::new(0.45, 0.48, 0.52)
            } else {
                Vec3::new(0.20, 0.23, 0.29)
            };
            push_rgba8(&mut rgba8, color, 1.0);
        }
    }
    insert_texture(engine, name, size, size, rgba8)
}

fn insert_texture(engine: &mut Engine, name: &str, width: u32, height: u32, rgba8: Vec<u8>) -> TextureHandle {
    engine
        .get_resource_mut::<RenderAssets>()
        .expect("RenderPlugin inserts RenderAssets")
        .insert_texture(TextureAsset {
            name: name.to_string(),
            width,
            height,
            rgba8,
            revision: 0,
        })
}

fn push_rgba8(out: &mut Vec<u8>, color: Vec3, alpha: f32) {
    let color = color.clamp(Vec3::ZERO, Vec3::ONE);
    out.extend_from_slice(&[
        (color.x * 255.0 + 0.5) as u8,
        (color.y * 255.0 + 0.5) as u8,
        (color.z * 255.0 + 0.5) as u8,
        (alpha.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
    ]);
}

fn spawn_box(engine: &mut Engine, name: &str, position: Vec3, size: Vec3, material: Material) -> Actor {
    spawn_layered_shape(engine, name, PrimitiveShape::Cube, position, size, material, u32::MAX)
}

fn spawn_layered_shape(
    engine: &mut Engine,
    name: &str,
    primitive: PrimitiveShape,
    position: Vec3,
    size: Vec3,
    material: Material,
    layer_mask: u32,
) -> Actor {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        })
        .with(RenderLayers { mask: layer_mask })
        .bundle(RenderBundle {
            shape: Shape { primitive, size },
            material,
            renderable: Renderable {
                visible: true,
                ..Renderable::default()
            },
        })
        .build()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    AppBuilder::new()
        .insert_resource(RenderSettings {
            title: "Vetrace Reflection Lab - Cubemap + Parallax + SSR".to_string(),
            width: 1440,
            height: 900,
            cursor_grab: true,
            cursor_visible: false,
            anti_aliasing_mode: AntiAliasingMode::Fxaa,
            ambient_occlusion_mode: AmbientOcclusionMode::Ssao,
            ssao_radius_pixels: 7.0,
            ssao_intensity: 1.15,
            ssao_bias: 0.0025,
            ssao_sample_count: 12,
            ssao_blur_radius: 1.5,
            shadow_map_size: 2048,
            shadow_soft_radius: 2.5,
            shadow_filter_mode: ShadowFilterMode::Pcss,
            shadow_pcf_quality: 3,
            reflection_max_capture_resolution: 1024,
            reflection_capture_faces_per_frame: 2,
            reflection_prefilter_sample_count: 128,
            reflection_capture_probe_budget_per_frame: 2,
            reflection_prefilter_mips_per_frame: 2,
            reflection_max_resident_runtime_probes: 8,
            reflection_probe_grid_cell_size: 8.0,
            ..RenderSettings::default()
        })
        .add_plugin(RenderPlugin::new())
        .run_until_stopped(CubemapEnvironmentExample::default(), None, 1.0 / 60.0)
}
