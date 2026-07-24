use super::*;

pub(super) fn extract_entity_renderables(
    engine: &Engine,
    entity: Entity,
    transform: GlobalTransform,
    assets: Option<&RenderAssets>,
    baked_lighting: Option<&BakedLightingScene>,
    scene: &mut SceneExtraction,
) {
    let renderable_visible = engine
        .raw_world()
        .get::<Renderable>(entity)
        .map(|renderable| renderable.visible)
        .unwrap_or(true);
    if !renderable_visible {
        return;
    }

    let material = material_for(engine, entity, assets);
    if let Some(emitter) = engine.raw_world().get::<EmissiveLightEmitter>(entity) {
        push_emissive_point_lights(emitter, &material, &transform, &mut scene.point_lights);
    }

    if extract_overlay(engine, entity, &material, scene) {
        return;
    }
    extract_sprite(engine, entity, &transform, &material, scene);
    extract_render_object(
        engine,
        entity,
        transform,
        material,
        assets,
        baked_lighting,
        scene,
    );
}

fn extract_overlay(
    engine: &Engine,
    entity: Entity,
    material: &Material,
    scene: &mut SceneExtraction,
) -> bool {
    let Some(overlay) = engine.raw_world().get::<ScreenSpaceRect>(entity) else {
        return false;
    };

    #[cfg(feature = "egui_render")]
    let draw_overlay_rect = !engine.raw_world().has::<vetrace_ui::UIScreenSpace>(entity);
    #[cfg(not(feature = "egui_render"))]
    let draw_overlay_rect = true;

    if draw_overlay_rect {
        let name = engine
            .raw_world()
            .get::<Name>(entity)
            .map(|name| name.0.clone());
        scene.overlays.push(RenderOverlayRect {
            entity,
            name,
            rect: overlay.clone(),
            material: material.clone(),
        });
    }
    true
}

fn extract_sprite(
    engine: &Engine,
    entity: Entity,
    transform: &GlobalTransform,
    material: &Material,
    scene: &mut SceneExtraction,
) {
    if let Some(sprite) = engine.raw_world().get::<Sprite3D>(entity) {
        scene.sprites.push(RenderSprite {
            entity,
            transform: transform.clone(),
            sprite: sprite.clone(),
            material: material.clone(),
        });
    }
}

fn extract_render_object(
    engine: &Engine,
    entity: Entity,
    transform: GlobalTransform,
    material: Material,
    assets: Option<&RenderAssets>,
    baked_lighting: Option<&BakedLightingScene>,
    scene: &mut SceneExtraction,
) {
    let shape = engine.raw_world().get::<Shape>(entity).cloned();
    let renderable = engine.raw_world().get::<Renderable>(entity);
    let obj_mesh = engine.raw_world().get::<ObjMesh>(entity);
    let mesh = renderable
        .and_then(|renderable| renderable.mesh)
        .or_else(|| obj_mesh.map(|mesh| mesh.mesh));
    if renderable.is_none() && shape.is_none() && mesh.is_none() {
        return;
    }

    let name = engine
        .raw_world()
        .get::<Name>(entity)
        .map(|name| name.0.clone());
    let custom_shader = engine
        .raw_world()
        .get::<CustomShaderMaterial>(entity)
        .cloned();
    let outline = engine.raw_world().get::<Outline>(entity).cloned();
    let skin = render_skin_for(engine, entity, &transform);
    let mut geometry_revision = skin.as_ref().map(RenderSkin::signature).unwrap_or(0);
    if let Some(mesh_asset) = mesh.and_then(|handle| assets.and_then(|assets| assets.meshes.get(&handle.0))) {
        geometry_revision ^= mesh_asset.revision.rotate_left(17);
    }
    let render_layers = engine
        .raw_world()
        .get::<RenderLayers>(entity)
        .map(|layers| layers.mask)
        .unwrap_or(ALL_RENDER_LAYERS);
    let wants_lightmap = engine
        .raw_world()
        .get::<BakedLightmapReceiver>(entity)
        .map(|receiver| receiver.enabled)
        .unwrap_or(false);
    let wants_probes = engine
        .raw_world()
        .get::<BakedLightProbeReceiver>(entity)
        .filter(|receiver| receiver.enabled)
        .map(|receiver| receiver.intensity.max(0.0));

    let mut object = RenderObject {
        entity,
        name,
        transform,
        shape,
        mesh,
        material,
        custom_shader,
        outline,
        skin,
        geometry_revision,
        render_layers,
        baked_lightmap: None,
        baked_probes: None,
    };
    let (baked_lightmap, baked_probes) = render_baked_lighting_for_object(
        baked_lighting,
        &object,
        assets,
        wants_lightmap,
        wants_probes,
    );
    object.baked_lightmap = baked_lightmap;
    object.baked_probes = baked_probes;
    scene.objects.push(object);
}
