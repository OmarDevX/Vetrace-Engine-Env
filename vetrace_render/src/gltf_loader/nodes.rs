use super::*;

pub(crate) fn spawn_node(
    engine: &mut Engine,
    node: gltf::Node<'_>,
    parent: Entity,
    buffers: &[gltf::buffer::Data],
    material_handles: &[MaterialHandle],
    default_material: MaterialHandle,
    options: &GltfLoadOptions,
    report: &mut GltfLoadReport,
    #[cfg(feature = "gltf_animation")]
    node_entities: &mut HashMap<usize, Entity>,
) -> Result<Entity> {
    let name = node
        .name()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("gltf_node_{}", node.index()));
    let entity = engine.spawn_actor(name.clone()).with(transform_from_gltf_node(&node)).build().entity();
    attach_child(engine, parent, entity);
    #[cfg(feature = "gltf_animation")]
    node_entities.insert(node.index(), entity);
    report.nodes_spawned += 1;

    let collision_intent = named_collision_intent(&name, options);

    if options.import_lights && attach_punctual_light(engine, entity, &node) {
        report.lights_loaded += 1;
    }

    if let Some(mesh) = node.mesh() {
        #[cfg(feature = "gltf_animation")]
        attach_initial_morph_weights(engine, entity, &node);
        let primitives: Vec<_> = mesh.primitives().collect();
        if primitives.len() == 1 {
            let primitive = primitives.into_iter().next().expect("len was checked");
            attach_primitive(
                engine,
                entity,
                &name,
                primitive,
                buffers,
                material_handles,
                default_material,
                options,
                collision_intent,
                report,
            )?;
        } else {
            for (primitive_index, primitive) in primitives.into_iter().enumerate() {
                let primitive_name = format!("{name}_primitive_{primitive_index}");
                let primitive_entity = engine.spawn_actor(primitive_name.clone()).with(Transform::default()).build().entity();
                attach_child(engine, entity, primitive_entity);
                report.nodes_spawned += 1;
                attach_primitive(
                    engine,
                    primitive_entity,
                    &primitive_name,
                    primitive,
                    buffers,
                    material_handles,
                    default_material,
                    options,
                    collision_intent,
                    report,
                )?;
            }
        }
    } else if let Some(intent) = collision_intent {
        // Empty named marker node, useful for box/sphere/capsule triggers authored
        // with only transform scale in DCC tools.
        engine.raw_world_mut().insert(entity, empty_collider_from_intent(&name, intent));
        mark_collision_entity(engine, entity, &name);
        report.collision_nodes_loaded += 1;
    }

    for child in node.children() {
        spawn_node(
            engine,
            child,
            entity,
            buffers,
            material_handles,
            default_material,
            options,
            report,
            #[cfg(feature = "gltf_animation")]
            node_entities,
        )?;
    }

    Ok(entity)
}

fn attach_punctual_light(engine: &mut Engine, entity: Entity, node: &gltf::Node<'_>) -> bool {
    let Some(light) = node.light() else { return false; };
    let color = {
        let c = light.color();
        Vec3::new(c[0], c[1], c[2])
    };
    let intensity = light.intensity().max(0.0);
    let range = light.range();
    match light.kind() {
        gltf::khr_lights_punctual::Kind::Directional => {
            // glTF punctual directional lights emit along the node's local -Z axis.
            // `build_render_frame` rotates this local direction by the node/global
            // transform, so store the local-space glTF direction here.
            engine.raw_world_mut().insert(entity, DirectionalLight {
                direction: Vec3::new(0.0, 0.0, -1.0),
                color,
                intensity,
                ..DirectionalLight::default()
            });
        }
        gltf::khr_lights_punctual::Kind::Point => {
            engine.raw_world_mut().insert(entity, PointLight {
                color,
                intensity,
                range,
                ..PointLight::default()
            });
        }
        gltf::khr_lights_punctual::Kind::Spot { inner_cone_angle, outer_cone_angle } => {
            engine.raw_world_mut().insert(entity, SpotLight {
                direction: Vec3::new(0.0, 0.0, -1.0),
                color,
                intensity,
                range,
                inner_cone_angle,
                outer_cone_angle,
                ..SpotLight::default()
            });
        }
    }
    true
}

fn attach_primitive(
    engine: &mut Engine,
    entity: Entity,
    name: &str,
    primitive: gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
    material_handles: &[MaterialHandle],
    default_material: MaterialHandle,
    options: &GltfLoadOptions,
    collision_intent: Option<CollisionIntent>,
    report: &mut GltfLoadReport,
) -> Result<()> {
    if primitive.mode() != gltf::mesh::Mode::Triangles {
        report.skipped_primitives += 1;
        return Ok(());
    }

    let wants_collision = collision_intent.is_some() || should_auto_static_mesh(options, collision_intent);
    let should_render = options.import_meshes && !collision_intent.map(|intent| intent.hide_render_mesh).unwrap_or(false);
    if !wants_collision && !should_render {
        return Ok(());
    }

    let mesh_asset = mesh_asset_from_primitive(name, &primitive, buffers, options.generate_missing_normals)?;
    if mesh_asset.vertices.is_empty() {
        report.skipped_primitives += 1;
        return Ok(());
    }

    if let Some(intent) = collision_intent {
        if let Some(collider) = collider_from_mesh_asset(name, &mesh_asset, intent) {
            engine.raw_world_mut().insert(entity, collider);
            mark_collision_entity(engine, entity, name);
            report.collision_nodes_loaded += 1;
        }
    } else if should_auto_static_mesh(options, collision_intent) {
        if let Some(collider) = auto_static_collider_from_mesh_asset(name, &mesh_asset) {
            engine.raw_world_mut().insert(entity, collider);
            mark_collision_entity(engine, entity, name);
            report.auto_colliders_loaded += 1;
        }
    }

    if should_render {
        let material = primitive
            .material()
            .index()
            .and_then(|index| material_handles.get(index).copied())
            .unwrap_or(default_material);
        let mesh = {
            let assets = render_assets_mut(engine);
            assets.insert_mesh(mesh_asset)
        };

        engine.raw_world_mut().insert(entity, Renderable { mesh: Some(mesh), material: Some(material), visible: true });
        engine.raw_world_mut().insert(entity, ObjMesh { mesh, material: Some(material), submesh_entities: Vec::new() });
        report.mesh_primitives_loaded += 1;
    }
    Ok(())
}

fn transform_from_gltf_node(node: &gltf::Node<'_>) -> Transform {
    let (translation, rotation, scale) = node.transform().decomposed();
    Transform {
        translation: Vec3::new(translation[0], translation[1], translation[2]),
        rotation: Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]).normalize(),
        scale: Vec3::new(scale[0], scale[1], scale[2]),
    }
}

fn attach_child(engine: &mut Engine, parent: Entity, child: Entity) {
    let Some(parent) = engine.actor(parent) else { return; };
    let Some(child) = engine.actor(child) else { return; };
    child.set_parent(engine, parent).expect("GLTF hierarchy must be acyclic and alive");
}
