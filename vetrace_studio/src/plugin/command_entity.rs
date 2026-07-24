use super::*;

impl StudioPlugin {
    pub(super) fn apply_entity_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::Select(entity) => vetrace_editor::select_editor_entity(engine, entity),
            StudioCommand::SetField { entity, component, path, value } => {
                let result = engine
                    .actor(entity)
                    .ok_or_else(|| "selected entity no longer exists".to_string())
                    .and_then(|actor| {
                        engine
                            .set_registered_component_field(actor, &component, &path, value)
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(()) => {
                        let status = format!("Edited {component}.{path}");
                        self.status = status.clone();
                        self.mark_scene_changed(status);
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::AddComponent { entity, component } => {
                let result = engine
                    .actor(entity)
                    .ok_or_else(|| "selected entity no longer exists".to_string())
                    .and_then(|actor| {
                        engine
                            .add_registered_component(actor, &component, None)
                            .map_err(|error| error.to_string())
                    });
                match result {
                    Ok(()) => {
                        let status = format!("Added {component}");
                        self.status = status.clone();
                        self.mark_scene_changed(status);
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::RemoveComponent { entity, component } => {
                let result = engine
                    .actor(entity)
                    .ok_or_else(|| "selected entity no longer exists".to_string())
                    .and_then(|actor| {
                        engine
                            .remove_registered_component(actor, &component)
                            .and_then(|removed| {
                                removed
                                    .then_some(())
                                    .ok_or_else(|| format!("component `{component}` was not removed"))
                            })
                    });
                match result {
                    Ok(()) => {
                        let status = format!("Removed {component}");
                        self.status = status.clone();
                        self.mark_scene_changed(status);
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::Rename { entity, name } => {
                if let Some(actor) = engine.actor(entity) {
                    match actor.set_name(engine, name) {
                        Ok(()) => self.mark_scene_changed("Renamed entity"),
                        Err(error) => self.log(error.to_string()),
                    }
                }
            }
            StudioCommand::SpawnEmpty => {
                let name = format!("Entity {}", self.spawn_index);
                self.spawn_index = self.spawn_index.saturating_add(1);
                let actor = engine.spawn_actor(name).build();
                vetrace_editor::select_editor_entity(engine, Some(actor.entity()));
                self.status = "Added empty entity".to_string();
                self.mark_scene_changed("Added empty entity");
            }
            StudioCommand::SpawnPrimitive(kind) => {
                let name = format!("{} {}", vetrace_primitives::primitive_display_name(kind), self.spawn_index);
                self.spawn_index = self.spawn_index.saturating_add(1);
                let actor = spawn_primitive_actor(
                    engine,
                    PrimitiveSpawnOptions {
                        name,
                        primitive: kind,
                        translation: Vec3::ZERO,
                        collider: PrimitiveColliderOptions::disabled(),
                        ..PrimitiveSpawnOptions::default()
                    },
                );
                vetrace_editor::select_editor_entity(engine, Some(actor.entity()));
                let status = format!("Added {}", vetrace_primitives::primitive_display_name(kind));
                self.status = status.clone();
                self.mark_scene_changed(status);
            }
            #[cfg(feature = "render_2d")]
            StudioCommand::SpawnSprite2D => {
                self.spawn_sprite_2d(engine, None, None);
            }
            #[cfg(feature = "render_2d")]
            StudioCommand::SpawnSprite2DFromAsset { path, screen_position } => {
                self.spawn_sprite_2d(engine, Some(path), Some(screen_position));
            }
            #[cfg(feature = "render_2d")]
            StudioCommand::SetViewportMode(mode) => {
                if let Some(state) = engine.get_resource_mut::<EditorState>() {
                    state.viewport_mode = mode;
                    state.status = match mode {
                        vetrace_editor::EditorViewportMode::TwoD => "2D viewport".to_owned(),
                        vetrace_editor::EditorViewportMode::ThreeD => "3D viewport".to_owned(),
                    };
                }
                self.status = match mode {
                    vetrace_editor::EditorViewportMode::TwoD => "Switched to 2D viewport".to_owned(),
                    vetrace_editor::EditorViewportMode::ThreeD => "Switched to 3D viewport".to_owned(),
                };
            }
            StudioCommand::DeleteSelected => {
                let selected = engine
                    .get_resource::<EditorState>()
                    .and_then(EditorState::selected_primary);
                if let Some(entity) = selected {
                    vetrace_editor::select_editor_entity(engine, None);
                    if let Some(actor) = engine.actor(entity) {
                        actor.despawn(engine);
                        self.status = "Deleted entity".to_string();
                        self.mark_scene_changed("Deleted entity");
                    }
                }
            }
            _ => unreachable!("non-entity command routed to entity handler"),
        }
    }
}

#[cfg(feature = "render_2d")]
impl StudioPlugin {
    fn spawn_sprite_2d(
        &mut self,
        engine: &mut Engine,
        asset_path: Option<std::path::PathBuf>,
        screen_position: Option<[f32; 2]>,
    ) {
        let camera = engine.get_resource::<Camera2D>().cloned().unwrap_or_default();
        let position = screen_position.map_or(Vec2::ZERO, |screen| {
            let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
            camera.screen_to_world(
                Vec2::new(screen[0], screen[1]),
                Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32),
            )
        });

        let mut texture_handle = None;
        let mut size = Vec2::ONE;
        let project_root = self.project.root().to_path_buf();
        let canonical_project_root = std::fs::canonicalize(&project_root)
            .unwrap_or_else(|_| project_root.clone());
        let absolute_asset_path = asset_path.as_ref().map(|path| {
            let absolute = if path.is_absolute() {
                path.clone()
            } else {
                project_root.join(path)
            };
            std::fs::canonicalize(&absolute).unwrap_or(absolute)
        });
        let texture_path = absolute_asset_path
            .as_ref()
            .and_then(|path| path.strip_prefix(&canonical_project_root).ok())
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        if asset_path.is_some() && texture_path.is_none() {
            self.log("2D sprite texture is outside the project and will not be stored as an absolute scene dependency".to_owned());
        }
        let name = asset_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                let name = format!("Sprite 2D {}", self.spawn_index);
                self.spawn_index = self.spawn_index.saturating_add(1);
                name
            });

        if let Some(absolute) = absolute_asset_path.as_ref() {
            match image::open(absolute) {
                Ok(decoded) => {
                    let rgba = decoded.to_rgba8();
                    let (width, height) = rgba.dimensions();
                    let pixels_per_unit = camera.pixels_per_unit.max(0.0001);
                    size = Vec2::new(width.max(1) as f32, height.max(1) as f32) / pixels_per_unit;
                    if !engine.contains_resource::<RenderAssets>() {
                        engine.insert_resource(RenderAssets::default());
                    }
                    texture_handle = engine.get_resource_mut::<RenderAssets>().map(|assets| {
                        assets.insert_texture(TextureAsset {
                            name: absolute
                                .file_name()
                                .and_then(|file| file.to_str())
                                .unwrap_or("sprite_2d")
                                .to_owned(),
                            width: width.max(1),
                            height: height.max(1),
                            rgba8: rgba.into_raw(),
                            revision: 0,
                        })
                    });
                }
                Err(error) => self.log(format!(
                    "Could not load 2D sprite texture '{}': {error}",
                    absolute.display()
                )),
            }
        }

        let actor = spawn_sprite_2d_actor(
            engine,
            Sprite2DSpawnOptions {
                name,
                texture: texture_handle,
                texture_path,
                position,
                size,
                tint: Vec4::ONE,
                ..Sprite2DSpawnOptions::default()
            },
        );
        if let Some(state) = engine.get_resource_mut::<EditorState>() {
            state.viewport_mode = vetrace_editor::EditorViewportMode::TwoD;
        }
        vetrace_editor::select_editor_entity(engine, Some(actor.entity()));
        self.status = "Added Sprite 2D".to_owned();
        self.mark_scene_changed("Added Sprite 2D");
    }
}
