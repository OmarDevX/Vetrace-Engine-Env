use super::*;

pub fn load_gltf_scene(engine: &mut Engine, path: impl AsRef<Path>) -> Result<Entity> {
    Ok(load_gltf_scene_with_options(engine, path, GltfLoadOptions::default())?.root)
}

/// Actor-first GLTF scene loader.
pub fn load_gltf_actor(engine: &mut Engine, path: impl AsRef<Path>) -> Result<Actor> {
    let entity = load_gltf_scene(engine, path)?;
    engine.actor(entity).context("GLTF loader returned a dead root actor")
}

/// Convenience loader for map/blockout assets: imports visuals, authored collision
/// helper nodes, and auto static triangle-mesh colliders for visible meshes.
/// Use this only for static level geometry, not animated/dynamic props.
pub fn load_gltf_static_map(engine: &mut Engine, path: impl AsRef<Path>) -> Result<Entity> {
    Ok(load_gltf_scene_with_options(engine, path, GltfLoadOptions::static_map())?.root)
}

/// Actor-first static-map GLTF loader.
pub fn load_gltf_static_map_actor(engine: &mut Engine, path: impl AsRef<Path>) -> Result<Actor> {
    let entity = load_gltf_static_map(engine, path)?;
    engine.actor(entity).context("GLTF static-map loader returned a dead root actor")
}

pub fn load_gltf_scene_with_options(
    engine: &mut Engine,
    path: impl AsRef<Path>,
    options: GltfLoadOptions,
) -> Result<GltfLoadReport> {
    let path = path.as_ref();
    let abs = path.to_path_buf();
    let (document, buffers, images) = gltf::import(path).with_context(|| format!("import glTF `{}`", path.display()))?;
    let scene = select_scene(&document, options.scene_index)?;

    ensure_render_assets(engine);
    let default_material = {
        let assets = render_assets_mut(engine);
        assets.insert_material(Material { double_sided: false, ..Material::default() })
    };
    let texture_handles = if options.import_textures {
        import_textures(engine, &document, &images)
    } else {
        Vec::new()
    };
    let material_handles = if options.import_materials {
        import_materials(engine, &document, &texture_handles)
    } else {
        Vec::new()
    };

    let root_name = options
        .root_name
        .clone()
        .or_else(|| scene.name().map(ToOwned::to_owned))
        .or_else(|| path.file_stem().and_then(|stem| stem.to_str()).map(ToOwned::to_owned))
        .unwrap_or_else(|| "gltf_scene".to_string());

    let root_actor = engine
        .spawn_actor(root_name)
        .with(Transform::default())
        .tag("gltf")
        .source(path.display().to_string())
        .build();
    let root = root_actor.entity();

    let mut report = GltfLoadReport {
        root,
        path: abs,
        nodes_spawned: 1,
        mesh_primitives_loaded: 0,
        materials_loaded: material_handles.len() + 1,
        textures_loaded: texture_handles.iter().filter(|handle| handle.is_some()).count(),
        lights_loaded: 0,
        collision_nodes_loaded: 0,
        auto_colliders_loaded: 0,
        skipped_primitives: 0,
        #[cfg(feature = "gltf_animation")]
        animations_loaded: 0,
        #[cfg(feature = "gltf_animation")]
        animation_channels_loaded: 0,
    };

    #[cfg(feature = "gltf_animation")]
    let mut node_entities = HashMap::new();

    for node in scene.nodes() {
        spawn_node(
            engine,
            node,
            root,
            &buffers,
            &material_handles,
            default_material,
            &options,
            &mut report,
            #[cfg(feature = "gltf_animation")]
            &mut node_entities,
        )?;
    }

    #[cfg(feature = "gltf_animation")]
    if options.import_animations {
        import_skins(engine, &document, &buffers, &node_entities);
        import_animations(engine, &document, &buffers, root, &node_entities, &mut report)?;
    }

    // Make imported hierarchy immediately renderable even in apps that do not run
    // the optional hierarchy plugin before the first frame.
    propagate_global_transforms(engine);
    Ok(report)
}
