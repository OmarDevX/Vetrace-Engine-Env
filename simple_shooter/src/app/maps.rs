use super::*;

pub(crate) const LOBBY_SPAWNS: &[(f32, f32)] = &[(-9.0, -9.0), (9.0, 9.0), (-9.0, 9.0), (9.0, -9.0), (0.0, -10.0), (0.0, 10.0), (-10.0, 0.0), (10.0, 0.0)];
pub(crate) const SPAWN_GROUND_PROBE_UP: f32 = 2.0;
pub(crate) const SPAWN_GROUND_PROBE_DISTANCE: f32 = 100.0;
pub(crate) fn map_name(index: u8) -> String {
    external_map(index).map(|map| map.name).unwrap_or_else(|| "No Maps Installed".to_string())
}
pub(crate) fn normalize_map_index(index: u8) -> u8 {
    if (index as usize) < map_count() { index } else { 0 }
}
pub(crate) fn map_count() -> usize { external_maps().read().map(|maps| maps.iter().flatten().count()).unwrap_or(0) }
pub(crate) fn minimum_map_capacity() -> u16 {
    external_maps().read().ok()
        .and_then(|maps| maps.iter().flatten().map(|map| map.spawn_point_ids.len() as u16).filter(|count| *count > 0).min())
        .unwrap_or(DEFAULT_MAX_PLAYERS)
}
pub(crate) fn active_map_capacity(engine: &Engine) -> usize {
    match engine.get_resource::<ShooterMapState>().map(|state| state.active).unwrap_or(ShooterMapKind::None) {
        ShooterMapKind::Lobby | ShooterMapKind::None => LOBBY_SPAWNS.len(),
        ShooterMapKind::Game(_) => engine.get_resource::<ShooterMapState>().map(|state| state.spawn_points.len()).unwrap_or(0),
    }
}

pub(crate) fn activate_lobby_map(engine: &mut Engine) {
    if map_is_active(engine, ShooterMapKind::Lobby) { return; }
    clear_active_map(engine);
    spawn_map_floor(engine, "Lobby Floor", Vec3::new(0.0, -0.5, 0.0), Vec3::new(28.0, 1.0, 28.0), Vec3::new(0.055, 0.075, 0.105));
    spawn_map_walls(engine, 14.0, 14.0, Vec3::new(0.08, 0.16, 0.23));
    spawn_map_block(engine, "Lobby Center", Vec3::new(0.0, 0.5, 0.0), Vec3::new(4.5, 1.0, 4.5), Vec3::new(0.12, 0.32, 0.48));
    for (x, z) in [(-7.0, -7.0), (7.0, -7.0), (-7.0, 7.0), (7.0, 7.0)] {
        spawn_map_block(engine, "Lobby Cover", Vec3::new(x, 1.0, z), Vec3::new(2.2, 2.0, 2.2), Vec3::new(0.16, 0.23, 0.31));
    }
    set_active_map(engine, ShooterMapKind::Lobby);
    set_validated_spawn_points(engine, LOBBY_SPAWNS.iter().map(|(x, z)| Vec3::new(*x, PLAYER_HEIGHT * 0.5 + 0.15, *z)).collect());
    apply_baked_lighting_for_map(engine, ShooterMapKind::Lobby);
    rebuild_navigation_grid(engine);
}

pub(crate) fn activate_game_map(engine: &mut Engine, map_index: u8) -> bool {
    let map_index = normalize_map_index(map_index);
    let Some(map) = external_map(map_index) else {
        eprintln!("no playable maps are installed in simple_shooter/maps");
        activate_lobby_map(engine);
        set_map_validation_error(engine, "No playable maps are installed in simple_shooter/maps.".to_string());
        return false;
    };
    let kind = ShooterMapKind::Game(map_index);
    if map_is_active(engine, kind) {
        return engine.get_resource::<ShooterMapState>().map(|state| !state.spawn_points.is_empty()).unwrap_or(false);
    }
    clear_active_map(engine);
    let instance = match map.document.instantiate_with_assets(engine, &map.scene_path) {
            Ok((instance, textures)) => {
                let floor = instance.actors.iter().copied().filter_map(|actor| {
                    let transform = actor.get_component::<Transform>(engine)?;
                    Some((actor, transform.scale.x.abs() * transform.scale.z.abs()))
                }).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(actor, _)| actor);
                for actor in instance.actors.iter().copied() {
                    let _ = actor.insert(engine, ShooterMapGeometry);
                    let _ = actor.insert(engine, shooter_lightmap_receiver());
                    if Some(actor) != floor && actor.has::<Collider>(engine) {
                        let _ = actor.insert(engine, ShooterNavigationObstacle);
                    }
                }
                if textures.missing > 0 { eprintln!("map `{}` is missing {} texture(s)", map.name, textures.missing); }
                instance
            }
            Err(err) => {
                eprintln!("failed to instantiate hosted map `{}`: {err:#}", map.name);
                activate_lobby_map(engine);
                set_map_validation_error(engine, format!("Map `{}` could not be loaded: {err:#}", map.name));
                return false;
            }
        };

    let authored_positions = map.spawn_point_ids.iter().filter_map(|id| {
        let actor = instance.actor(id)?;
        actor.get_component::<GlobalTransform>(engine).map(|global| global.translation)
            .or_else(|| actor.get_component::<Transform>(engine).map(|transform| transform.translation))
    }).collect::<Vec<_>>();
    let valid = validate_spawn_points(engine, &authored_positions);
    let required = required_spawn_count(engine);
    if valid.len() < required {
        let reason = format!(
            "Map `{}` has {} valid spawn point(s), but {} are required. Add Spawn Point objects above collidable ground in vetrace_map_builder.",
            map.name, valid.len(), required,
        );
        eprintln!("{reason}");
        activate_lobby_map(engine);
        set_map_validation_error(engine, reason);
        return false;
    }
    set_active_map(engine, kind);
    set_validated_spawn_points(engine, valid);
    set_map_validation_error(engine, String::new());
    apply_baked_lighting_for_map(engine, kind);
    rebuild_navigation_grid(engine);
    true
}

pub(crate) fn validate_spawn_points(engine: &Engine, authored: &[Vec3]) -> Vec<Vec3> {
    let mut valid = Vec::new();
    for point in authored.iter().copied().filter(|point| point.is_finite()) {
        let origin = point + Vec3::Y * SPAWN_GROUND_PROBE_UP;
        let hit = raycast_colliders(engine, origin, -Vec3::Y, SPAWN_GROUND_PROBE_DISTANCE, |entity| {
            engine.actor(entity).map(|actor| actor.has::<ShooterMapGeometry>(engine)).unwrap_or(false)
        });
        let Some(hit) = hit else { continue; };
        let adjusted = hit.position + Vec3::Y * (PLAYER_HEIGHT * 0.5 + 0.15);
        if valid.iter().any(|old: &Vec3| old.distance_squared(adjusted) < 0.25) { continue; }
        valid.push(adjusted);
    }
    valid
}

pub(crate) fn required_spawn_count(engine: &Engine) -> usize {
    engine.actors_with::<ShooterPlayer>().into_iter()
        .filter(|(actor, _)| !actor.has::<ShooterBot>(engine))
        .count()
        .max(1)
}

pub(crate) fn set_validated_spawn_points(engine: &mut Engine, points: Vec<Vec3>) {
    if let Some(state) = engine.get_resource_mut::<ShooterMapState>() { state.spawn_points = points; }
}

pub(crate) fn set_map_validation_error(engine: &mut Engine, error: String) {
    if let Some(state) = engine.get_resource_mut::<ShooterMapState>() {
        state.validation_error = (!error.is_empty()).then_some(error);
    }
}

fn shooter_lightmap_receiver() -> BakedLightmapReceiver {
    BakedLightmapReceiver {
        preserve_local_lights: true,
        ..BakedLightmapReceiver::default()
    }
}

pub(crate) fn spawn_map_walls(engine: &mut Engine, half_x: f32, half_z: f32, color: Vec3) {
    spawn_map_block(engine, "North Wall", Vec3::new(0.0, 2.0, -half_z), Vec3::new(half_x * 2.0, 4.0, 1.0), color);
    spawn_map_block(engine, "South Wall", Vec3::new(0.0, 2.0, half_z), Vec3::new(half_x * 2.0, 4.0, 1.0), color);
    spawn_map_block(engine, "West Wall", Vec3::new(-half_x, 2.0, 0.0), Vec3::new(1.0, 4.0, half_z * 2.0), color);
    spawn_map_block(engine, "East Wall", Vec3::new(half_x, 2.0, 0.0), Vec3::new(1.0, 4.0, half_z * 2.0), color);
}

pub(crate) fn spawn_map_block(engine: &mut Engine, name: &str, position: Vec3, size: Vec3, color: Vec3) {
    spawn_map_cube(engine, name, position, size, color, true);
}

pub(crate) fn spawn_map_floor(engine: &mut Engine, name: &str, position: Vec3, size: Vec3, color: Vec3) {
    spawn_map_cube(engine, name, position, size, color, false);
}

pub(crate) fn spawn_map_cube(engine: &mut Engine, name: &str, position: Vec3, size: Vec3, color: Vec3, navigation_obstacle: bool) {
    let visible_color = color.max(Vec3::splat(0.14));
    let actor = engine.spawn_actor(name)
        .with(ShooterMapGeometry)
        // The Transform scale is the single size source for rendering, Rapier,
        // and raycasts. This prevents a visible cube and its collider from
        // drifting apart when either subsystem interprets primitive size.
        .with(Transform { translation: position, scale: size, ..Transform::default() })
        .with(Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE })
        .with(Material {
            base_color: visible_color,
            // Map geometry should not glow by default; emissive contribution fills
            // baked shadows and makes the lightmap appear flat.
            emissive: Vec3::ZERO,
            roughness: 0.68,
            metallic: 0.04,
            ..Material::default()
        })
        .with(Renderable { visible: true, ..Renderable::default() })
        .with(shooter_lightmap_receiver())
        .with(StaticBody::default())
        .with(Collider { shape: ColliderShape::Cube, half_extents: Vec3::splat(0.5), ..Collider::default() })
        .build();
    if navigation_obstacle { let _ = actor.insert(engine, ShooterNavigationObstacle); }
}


fn baked_lighting_path(kind: ShooterMapKind) -> std::path::PathBuf {
    let file_name = match kind {
        ShooterMapKind::Lobby => "lobby.vlight".to_string(),
        ShooterMapKind::Game(index) => format!("game_{index}.vlight"),
        ShooterMapKind::None => "none.vlight".to_string(),
    };
    let asset_probe = resolve_simple_shooter_asset_path("player_gradient.wgsl");
    let asset_root = asset_probe
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("assets"));
    asset_root.join("baked_lighting").join(file_name)
}

fn apply_baked_lighting_for_map(engine: &mut Engine, kind: ShooterMapKind) {
    if matches!(kind, ShooterMapKind::None) {
        unload_baked_lighting(engine);
        return;
    }
    // Hierarchical scene imports must have their final global transforms before
    // keys, probe bounds, and CPU ray-tracing geometry are generated.
    vetrace_core::propagate_global_transforms(engine);
    let path = baked_lighting_path(kind);
    let baked_settings = engine
        .get_resource::<ShooterBakedLightingSettings>()
        .copied()
        .unwrap_or_default();
    let runtime_mode = match baked_settings.graphics_profile {
        ShooterGraphicsProfile::HighQuality => BakedLightingRuntimeMode::HybridRealtimeDirect,
        ShooterGraphicsProfile::LowSpec | ShooterGraphicsProfile::Balanced => BakedLightingRuntimeMode::BakedOnly,
    };
    if baked_settings.force_bake {
        let mut config = BakedLightingBakeConfig::default();
        config.source_name = match kind {
            ShooterMapKind::Lobby => "simple_shooter/lobby".to_string(),
            ShooterMapKind::Game(index) => format!("simple_shooter/{}", map_name(index)),
            ShooterMapKind::None => "simple_shooter/none".to_string(),
        };
        match baked_settings.graphics_profile {
            ShooterGraphicsProfile::LowSpec => {
                config.lightmap_texels_per_unit = 4.0;
                config.lightmap_filter_radius = 0;
                config.probe_counts = [6, 3, 6];
                config.probe_rays = 32;
            }
            ShooterGraphicsProfile::Balanced => {
                config.lightmap_texels_per_unit = 8.0;
                config.lightmap_filter_radius = 1;
                config.probe_counts = [8, 4, 8];
                config.probe_rays = 48;
            }
            ShooterGraphicsProfile::HighQuality => {
                config.lightmap_texels_per_unit = 12.0;
                config.lightmap_filter_radius = 2;
                config.probe_counts = [10, 5, 10];
                config.probe_rays = 64;
            }
        }
        match bake_and_save_baked_lighting(engine, &path, &config) {
            Ok(report) => {
                set_baked_lighting_runtime_mode(engine, runtime_mode);
                println!(
                    "baked lighting `{}`: {} receiver(s), {} triangle(s), {} probes, {}x{} atlas, {}..{} texel receiver tiles, {} bytes; runtime {:?}",
                    path.display(),
                    report.baked_receiver_count,
                    report.triangle_count,
                    report.probe_count,
                    report.atlas_width,
                    report.atlas_height,
                    report.min_lightmap_resolution,
                    report.max_lightmap_resolution,
                    report.output_bytes,
                    runtime_mode,
                );
            }
            Err(error) => {
                unload_baked_lighting(engine);
                eprintln!("failed to bake lighting `{}`: {error}", path.display());
            }
        }
        return;
    }

    match load_baked_lighting(engine, &path) {
        Ok(()) => {
            set_baked_lighting_runtime_mode(engine, runtime_mode);
            println!(
                "loaded baked lighting `{}` in {:?} mode (press B to inspect Lightmap/UV2/Probes)",
                path.display(),
                runtime_mode,
            );
        }
        Err(error) => {
            unload_baked_lighting(engine);
            eprintln!(
                "baked lighting unavailable for this map (`{}`): {error}. Run Simple Shooter once with --bake-lighting to create it; normal runtime will not rebake.",
                path.display(),
            );
        }
    }
}

pub(crate) fn rebuild_navigation_grid(engine: &mut Engine) {
    let settings = engine.get_resource::<PathfindingSettings>().copied().unwrap_or_default();
    let floor = engine.actors_with::<ShooterMapGeometry>().into_iter().filter_map(|(actor, _)| {
        if actor.has::<ShooterNavigationObstacle>(engine) { return None; }
        actor.get_component::<Transform>(engine).cloned()
    }).max_by(|a, b| (a.scale.x * a.scale.z).total_cmp(&(b.scale.x * b.scale.z)));
    let Some(floor) = floor else { return; };
    let inset = settings.agent_clearance + settings.cell_size * 0.5;
    let half = Vec2::new(floor.scale.x.abs(), floor.scale.z.abs()) * 0.5 - Vec2::splat(inset);
    let center = Vec2::new(floor.translation.x, floor.translation.z);
    let mut grid = NavigationGrid::new(center - half, center + half, settings.cell_size).with_diagonal_movement(settings.allow_diagonal);
    let obstacles = engine.actors_with::<ShooterNavigationObstacle>().into_iter().filter_map(|(actor, _)| actor.get_component::<Transform>(engine).cloned()).collect::<Vec<_>>();
    for obstacle in obstacles {
        grid.block_aabb(Vec2::new(obstacle.translation.x, obstacle.translation.z), Vec2::new(obstacle.scale.x, obstacle.scale.z), settings.agent_clearance);
    }
    if let Some(world) = engine.get_resource_mut::<PathfindingWorld>() { world.set_active_grid(grid); }
}

pub(crate) fn clear_active_map(engine: &mut Engine) {
    let actors = engine.actors_with::<ShooterMapGeometry>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors { actor.despawn(engine); }
    set_active_map(engine, ShooterMapKind::None);
    unload_baked_lighting(engine);
    set_validated_spawn_points(engine, Vec::new());
    if let Some(world) = engine.get_resource_mut::<PathfindingWorld>() { world.clear(); }
}

pub(crate) fn clear_all_shooter_players(engine: &mut Engine) {
    despawn_all_player_weapons(engine);
    let players = engine.actors_with::<ShooterPlayer>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in players { actor.despawn(engine); }
    let labels = engine.actors_with::<PlayerNameLabel>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in labels { actor.despawn(engine); }
    despawn_orphan_outline_shells(engine);
}

pub(crate) fn clear_transient_gameplay_visuals(engine: &mut Engine) {
    let trails = engine.actors_with::<BulletTrail>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in trails { actor.despawn(engine); }
    let flashes = engine.actors_with::<MuzzleFlash>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in flashes { actor.despawn(engine); }
    set_crosshair_visible(engine, false);
    clear_killcam(engine);
    clear_health_feedback(engine);
}

pub(crate) fn set_crosshair_visible(engine: &mut Engine, visible: bool) {
    let actors = engine.actors_with::<CrosshairPart>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors {
        if let Some(renderable) = actor.get_component_mut::<Renderable>(engine) { renderable.visible = visible; }
    }
}

pub(crate) fn map_is_active(engine: &Engine, kind: ShooterMapKind) -> bool {
    engine.get_resource::<ShooterMapState>().map(|state| state.active == kind).unwrap_or(false)
}

pub(crate) fn set_active_map(engine: &mut Engine, kind: ShooterMapKind) {
    if let Some(state) = engine.get_resource_mut::<ShooterMapState>() { state.active = kind; }
    else { engine.insert_resource(ShooterMapState { active: kind, ..ShooterMapState::default() }); }
}

pub(crate) fn spawn_position_for_active_map(engine: &Engine, id: u64) -> Vec3 {
    let points = engine.get_resource::<ShooterMapState>().map(|state| state.spawn_points.clone()).unwrap_or_default();
    if !points.is_empty() { return points[id.saturating_sub(SERVER_AUTHORITY_ID) as usize % points.len()]; }
    let slot = id.saturating_sub(SERVER_AUTHORITY_ID) as usize % LOBBY_SPAWNS.len();
    spawn_position_from_points(LOBBY_SPAWNS, slot)
}

pub(crate) fn spawn_position_for_slot(engine: &Engine, slot: usize) -> Vec3 {
    let points = engine.get_resource::<ShooterMapState>().map(|state| state.spawn_points.clone()).unwrap_or_default();
    if !points.is_empty() { return points[slot % points.len()]; }
    spawn_position_from_points(LOBBY_SPAWNS, slot)
}

pub(crate) fn spawn_position_from_points(points: &[(f32, f32)], slot: usize) -> Vec3 {
    let slot = slot % points.len();
    let (x, z) = points[slot];
    Vec3::new(x, PLAYER_HEIGHT * 0.5 + 0.15, z)
}
