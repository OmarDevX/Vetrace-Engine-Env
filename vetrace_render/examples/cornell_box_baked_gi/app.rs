// Cornell Box application state, controls, and update loop.

struct CornellBoxExample {
    options: CornellOptions,
    probe_sphere: Actor,
    elapsed: f32,
    animate_probe_sphere: bool,
}

impl CornellBoxExample {
    fn new(options: CornellOptions) -> Self {
        Self {
            options,
            probe_sphere: Actor::INVALID,
            elapsed: 0.0,
            animate_probe_sphere: true,
        }
    }

    fn setup_scene(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource(Camera {
            position: Vec3::new(0.0, 2.55, 8.2),
            target: Vec3::new(0.0, 2.35, -0.15),
            fov_y_radians: 42.0_f32.to_radians(),
            near: 0.05,
            far: 40.0,
            ..Camera::default()
        });
        engine.insert_resource(PostProcessing {
            exposure: 1.08,
            gamma: 2.2,
            tone_mapper: ToneMapper::Neutral,
            ..PostProcessing::default()
        });

        // Use plausible Cornell-box albedos instead of neon-saturated walls so
        // the scene reads more like a conventional reference render while still
        // showing clear red/green color bleeding.
        let white = Vec3::splat(0.73);
        let red = Vec3::new(0.63, 0.065, 0.05);
        let green = Vec3::new(0.14, 0.45, 0.091);

        spawn_static_plane(
            engine,
            "Cornell Floor",
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::new(ROOM_WIDTH, 0.0, ROOM_DEPTH),
            white,
            1.25,
        );
        spawn_static_plane(
            engine,
            "Cornell Ceiling",
            Vec3::new(0.0, ROOM_HEIGHT, 0.0),
            Quat::from_rotation_x(PI),
            Vec3::new(ROOM_WIDTH, 0.0, ROOM_DEPTH),
            white,
            1.25,
        );
        spawn_static_plane(
            engine,
            "Cornell Back Wall",
            Vec3::new(0.0, ROOM_HEIGHT * 0.5, -ROOM_DEPTH * 0.5),
            Quat::from_rotation_x(FRAC_PI_2),
            Vec3::new(ROOM_WIDTH, 0.0, ROOM_HEIGHT),
            white,
            1.25,
        );
        spawn_static_plane(
            engine,
            "Cornell Red Wall",
            Vec3::new(-ROOM_WIDTH * 0.5, ROOM_HEIGHT * 0.5, 0.0),
            Quat::from_rotation_z(-FRAC_PI_2),
            Vec3::new(ROOM_HEIGHT, 0.0, ROOM_DEPTH),
            red,
            1.25,
        );
        spawn_static_plane(
            engine,
            "Cornell Green Wall",
            Vec3::new(ROOM_WIDTH * 0.5, ROOM_HEIGHT * 0.5, 0.0),
            Quat::from_rotation_z(FRAC_PI_2),
            Vec3::new(ROOM_HEIGHT, 0.0, ROOM_DEPTH),
            green,
            1.25,
        );

        spawn_static_cube(
            engine,
            "Cornell Tall Box",
            Vec3::new(-0.82, 1.35, -0.72),
            Quat::from_rotation_y(18.0_f32.to_radians()),
            Vec3::new(1.35, 2.70, 1.35),
            white,
            1.1,
        );
        spawn_static_cube(
            engine,
            "Cornell Short Box",
            Vec3::new(0.88, 0.68, 0.46),
            Quat::from_rotation_y(-17.0_f32.to_radians()),
            Vec3::new(1.55, 1.36, 1.55),
            white,
            1.1,
        );

        // Canonical Cornell illumination: a finite rectangular ceiling emitter.
        // The visible material is bright for display, while BakedRectAreaLight
        // performs deterministic stratified direct-light and soft-shadow sampling
        // during the explicit CPU bake. It has no normal per-frame light cost.
        engine
            .spawn_actor("Cornell Rectangular Ceiling Emitter")
            .with(Transform {
                translation: Vec3::new(0.0, ROOM_HEIGHT - 0.025, -0.18),
                rotation: Quat::from_rotation_x(PI),
                scale: Vec3::ONE,
            })
            .bundle(RenderBundle {
                shape: Shape {
                    primitive: PrimitiveShape::Plane,
                    size: Vec3::new(1.35, 0.0, 0.92),
                },
                material: Material {
                    base_color: Vec3::new(0.90, 0.88, 0.84),
                    emissive: Vec3::new(4.6, 4.35, 4.05),
                    roughness: 0.92,
                    metallic: 0.0,
                    ..Material::default()
                },
                renderable: Renderable { visible: true, ..Renderable::default() },
            })
            .with(BakedRectAreaLight {
                color: Vec3::new(1.0, 0.90, 0.76),
                intensity: self.options.area_light_intensity,
                width: 1.35,
                height: 0.92,
                samples: self.options.area_light_samples,
                two_sided: false,
                enabled: true,
            })
            .with(BakedLightmapReceiver {
                resolution_scale: 1.0,
                ..BakedLightmapReceiver::default()
            })
            .build();

        // This moving object is intentionally excluded from the lightmap bake.
        // Its diffuse lighting comes from trilinearly interpolated probes.
        self.probe_sphere = engine
            .spawn_actor("Moving Probe Test Sphere")
            .with(Transform {
                translation: Vec3::new(0.0, 1.15, 1.78),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            })
            .bundle(RenderBundle {
                shape: Shape {
                    primitive: PrimitiveShape::Sphere,
                    size: Vec3::splat(0.52),
                },
                material: Material {
                    // A physically plausible diffuse white. Values near 1.0
                    // leave too little display headroom to reveal colored GI.
                    base_color: Vec3::splat(0.76),
                    roughness: 0.68,
                    metallic: 0.0,
                    ..Material::default()
                },
                renderable: Renderable { visible: true, ..Renderable::default() },
            })
            .with(BakedLightProbeReceiver {
                enabled: true,
                intensity: self.options.probe_intensity,
            })
            .build();

        if self.options.bake {
            let config = cornell_bake_config(&self.options);
            println!("Cornell Box: baking to {}", self.options.baked_path.display());
            let report = bake_and_save_baked_lighting(engine, &self.options.baked_path, &config)?;
            println!(
                "Cornell Box baked: {} receiver(s), {} triangle(s), {} probes, {}x{} atlas, tiles {}..{}, {} bounce(es), {} bytes",
                report.baked_receiver_count,
                report.triangle_count,
                report.probe_count,
                report.atlas_width,
                report.atlas_height,
                report.min_lightmap_resolution,
                report.max_lightmap_resolution,
                config.indirect_bounces,
                report.output_bytes,
            );
        } else {
            load_baked_lighting(engine, &self.options.baked_path).map_err(|error| {
                format!(
                    "failed to load `{}`: {error}. Run the example once with --bake-lighting",
                    self.options.baked_path.display(),
                )
            })?;
            println!("Cornell Box: loaded {}", self.options.baked_path.display());
        }

        let runtime_mode = if self.options.start_hybrid {
            BakedLightingRuntimeMode::HybridRealtimeDirect
        } else {
            BakedLightingRuntimeMode::BakedOnly
        };
        self.apply_runtime_mode(engine, runtime_mode);
        print_controls(runtime_mode);
        Ok(())
    }

    fn apply_runtime_mode(&self, engine: &mut Engine, mode: BakedLightingRuntimeMode) {
        set_baked_lighting_runtime_mode(engine, mode);
    }

    fn handle_input(&mut self, engine: &mut Engine) {
        let (quit, cycle_debug, toggle_mode, toggle_animation, exposure_down, exposure_up, cycle_tonemap) = engine
            .get_resource::<InputState>()
            .map(|input| {
                (
                    input.quit_requested() || input.was_key_pressed("Escape"),
                    input.was_key_pressed("B"),
                    input.was_key_pressed("M"),
                    input.was_key_pressed("Space"),
                    input.was_key_pressed("J"),
                    input.was_key_pressed("K"),
                    input.was_key_pressed("T"),
                )
            })
            .unwrap_or_default();

        if quit {
            engine.stop();
        }
        if cycle_debug {
            let mode = cycle_baked_lighting_debug_mode(engine);
            let markers = engine.actors_with::<BakedLightProbeDebugMarker>().len();
            println!("Cornell Box debug: {mode:?} ({markers} probe marker(s))");
        }
        if toggle_mode {
            let next = match baked_lighting_runtime_mode(engine) {
                BakedLightingRuntimeMode::BakedOnly => {
                    BakedLightingRuntimeMode::HybridRealtimeDirect
                }
                BakedLightingRuntimeMode::HybridRealtimeDirect => {
                    BakedLightingRuntimeMode::BakedOnly
                }
            };
            self.apply_runtime_mode(engine, next);
            println!("Cornell Box runtime lighting: {next:?}");
        }
        if toggle_animation {
            self.animate_probe_sphere = !self.animate_probe_sphere;
            println!(
                "Cornell Box probe sphere animation: {}",
                if self.animate_probe_sphere { "running" } else { "paused" },
            );
        }
        if let Some(post) = engine.get_resource_mut::<PostProcessing>() {
            let mut changed = false;
            if exposure_down {
                post.exposure = (post.exposure * 0.9).clamp(0.05, 16.0);
                changed = true;
            }
            if exposure_up {
                post.exposure = (post.exposure * 1.1).clamp(0.05, 16.0);
                changed = true;
            }
            if cycle_tonemap {
                post.tone_mapper = post.tone_mapper.next();
                changed = true;
            }
            if changed {
                println!(
                    "Cornell Box post process: exposure {:.3}, tone mapper {:?}",
                    post.exposure,
                    post.tone_mapper,
                );
            }
        }
    }
}

impl App for CornellBoxExample {
    fn setup(&mut self, engine: &mut Engine) {
        if let Err(error) = self.setup_scene(engine) {
            eprintln!("Cornell Box setup failed: {error}");
            engine.stop();
        }
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.handle_input(engine);
        self.elapsed += dt.max(0.0);
        if self.animate_probe_sphere && self.probe_sphere.is_alive(engine) {
            let x = (self.elapsed * 0.62).sin() * 1.78;
            let y = 1.12 + (self.elapsed * 1.15).sin() * 0.12;
            let _ = self
                .probe_sphere
                .set_position(engine, Vec3::new(x, y, 1.72));
        }
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.time_seconds = self.elapsed;
        }
    }
}
