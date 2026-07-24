use super::*;

pub(super) fn extract_entity_view_lights_and_environment(
    engine: &Engine,
    entity: Entity,
    transform: &GlobalTransform,
    assets: Option<&RenderAssets>,
    max_reflection_capture_resolution: u32,
    scene: &mut SceneExtraction,
) {
    extract_render_texture_view(engine, entity, transform, scene);
    extract_lights(engine, entity, transform, scene);

    if let Some(value) = engine.raw_world().get::<Atmosphere>(entity) {
        scene.atmosphere = Some(value.clone());
    }
    if let Some(value) = engine.raw_world().get::<VolumetricFog>(entity) {
        scene.fog = Some(value.clone());
    }
    if let Some(probe) = engine.raw_world().get::<ReflectionProbe>(entity) {
        extract_reflection_probe(
            entity,
            probe,
            transform,
            assets,
            max_reflection_capture_resolution,
            &mut scene.reflection_probes,
        );
    }
}

fn extract_render_texture_view(
    engine: &Engine,
    entity: Entity,
    transform: &GlobalTransform,
    scene: &mut SceneExtraction,
) {
    let Some(view) = engine.raw_world().get::<RenderTextureCamera>(entity) else {
        return;
    };
    if !view.enabled || view.target_name.trim().is_empty() {
        return;
    }

    let forward = (transform.rotation * Vec3::NEG_Z).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 {
        forward
    } else {
        Vec3::NEG_Z
    };
    let up = (transform.rotation * Vec3::Y).normalize_or_zero();
    let up = if up.length_squared() > 0.0 { up } else { Vec3::Y };
    scene.render_texture_views.push(RenderTextureView {
        source_entity: entity,
        target_name: view.target_name.trim().to_string(),
        width: view.width.clamp(16, 4096),
        height: view.height.clamp(16, 4096),
        clear_color: view.clear_color,
        layer_mask: view.layer_mask,
        order: view.order,
        camera: Camera {
            position: transform.translation,
            target: transform.translation + forward,
            up,
            fov_y_radians: view
                .fov_y_radians
                .clamp(1.0_f32.to_radians(), 179.0_f32.to_radians()),
            near: view.near.max(0.001),
            far: view.far.max(view.near.max(0.001) + 0.001),
        },
    });
}

fn extract_lights(
    engine: &Engine,
    entity: Entity,
    transform: &GlobalTransform,
    scene: &mut SceneExtraction,
) {
    if let Some(light) = engine.raw_world().get::<DirectionalLight>(entity) {
        scene.directional_lights.push(RenderDirectionalLight {
            direction: rotated_direction(transform.rotation, light.direction),
            color: light.color,
            intensity: light.intensity,
            shadow_mode: light.shadow_mode,
        });
    }
    if let Some(light) = engine.raw_world().get::<PointLight>(entity) {
        scene.point_lights.push(RenderPointLight {
            position: transform.translation,
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            shadow_mode: light.shadow_mode,
        });
    }
    if let Some(light) = engine.raw_world().get::<SpotLight>(entity) {
        scene.spot_lights.push(RenderSpotLight {
            position: transform.translation,
            direction: rotated_direction(transform.rotation, light.direction),
            color: light.color,
            intensity: light.intensity,
            range: light.range,
            inner_cone_angle: light.inner_cone_angle,
            outer_cone_angle: light.outer_cone_angle,
            shadow_mode: light.shadow_mode,
        });
    }
}

fn extract_reflection_probe(
    entity: Entity,
    probe: &ReflectionProbe,
    transform: &GlobalTransform,
    assets: Option<&RenderAssets>,
    max_reflection_capture_resolution: u32,
    output: &mut Vec<RenderReflectionProbe>,
) {
    let primary = valid_cubemap_handle(assets, probe.primary);
    let secondary = valid_cubemap_handle(assets, probe.secondary);
    let captures_scene = !matches!(probe.capture_mode, ReflectionProbeCaptureMode::Imported);
    if !probe.enabled || (primary.is_none() && secondary.is_none() && !captures_scene) {
        return;
    }

    let absolute_scale = transform.scale.abs();
    let half_extents = (probe.half_extents.abs() * absolute_scale).max(Vec3::splat(0.001));
    let blend_distance = (probe.blend_distance.max(0.0) * absolute_scale.min_element())
        .min(half_extents.min_element());
    let probe_to_world =
        Mat4::from_rotation_translation(transform.rotation, transform.translation);
    let capture_position_local = probe.capture_offset * transform.scale;
    output.push(RenderReflectionProbe {
        entity,
        primary,
        secondary,
        transition: probe.transition.clamp(0.0, 1.0),
        world_to_probe: probe_to_world.inverse(),
        half_extents,
        capture_position_local,
        blend_distance,
        intensity: probe.intensity.max(0.0),
        priority: probe.priority,
        parallax_mode: probe.parallax_mode,
        capture_mode: probe.capture_mode,
        capture_resolution: probe
            .capture_resolution
            .clamp(32, max_reflection_capture_resolution)
            .next_power_of_two()
            .min(max_reflection_capture_resolution),
        capture_near: probe.capture_near.max(0.001),
        capture_far: probe
            .capture_far
            .max(probe.capture_near.max(0.001) + 0.01),
        transition_seconds: probe.transition_seconds.max(0.0),
        update_interval_seconds: probe.update_interval_seconds.max(0.0),
        capture_revision: probe.capture_revision,
        capture_priority: probe.capture_priority,
        invalidation_mode: probe.invalidation_mode,
        invalidation_delay_seconds: probe.invalidation_delay_seconds.max(0.0),
        capture_transparent: probe.capture_transparent,
        capture_shadows: probe.capture_shadows,
        capture_custom_materials: probe.capture_custom_materials,
        probe_to_world,
        capture_position_world: probe_to_world.transform_point3(capture_position_local),
        include_layers: probe.include_layers,
        exclude_layers: probe.exclude_layers,
        capture_include_layers: probe.capture_include_layers,
        capture_exclude_layers: probe.capture_exclude_layers,
    });
}
