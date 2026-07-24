// Cornell Box geometry, materials, and bake configuration.

fn spawn_static_plane(
    engine: &mut Engine,
    name: &str,
    position: Vec3,
    rotation: Quat,
    size: Vec3,
    color: Vec3,
    resolution_scale: f32,
) {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation,
            scale: Vec3::ONE,
        })
        .bundle(RenderBundle {
            shape: Shape { primitive: PrimitiveShape::Plane, size },
            material: diffuse_material(color),
            renderable: Renderable { visible: true, ..Renderable::default() },
        })
        .with(BakedLightmapReceiver {
            resolution_scale,
            static_lighting_only: true,
            preserve_local_lights: false,
            enabled: true,
        })
        .build();
}

fn spawn_static_cube(
    engine: &mut Engine,
    name: &str,
    position: Vec3,
    rotation: Quat,
    size: Vec3,
    color: Vec3,
    resolution_scale: f32,
) {
    engine
        .spawn_actor(name)
        .with(Transform {
            translation: position,
            rotation,
            scale: Vec3::ONE,
        })
        .bundle(RenderBundle {
            shape: Shape { primitive: PrimitiveShape::Cube, size },
            material: diffuse_material(color),
            renderable: Renderable { visible: true, ..Renderable::default() },
        })
        .with(BakedLightmapReceiver {
            resolution_scale,
            static_lighting_only: true,
            preserve_local_lights: false,
            enabled: true,
        })
        .build();
}

fn diffuse_material(color: Vec3) -> Material {
    Material {
        base_color: color,
        roughness: 0.82,
        metallic: 0.0,
        ..Material::default()
    }
}

fn cornell_bake_config(options: &CornellOptions) -> BakedLightingBakeConfig {
    BakedLightingBakeConfig {
        source_name: "vetrace-cornell-box".to_string(),
        lightmap_resolution: 96,
        lightmap_texels_per_unit: 20.0,
        lightmap_filter_radius: 3,
        atlas_padding: 8,
        probe_counts: [9, 7, 9],
        probe_rays: 384,
        probe_bounds_padding: 0.02,
        environment_radiance: Vec3::splat(0.0008),
        indirect_bounces: options.indirect_bounces,
        indirect_bounce_decay: options.indirect_bounce_decay,
        indirect_intensity: options.indirect_intensity,
        lightmap_intensity: options.lightmap_intensity,
        max_baked_radiance: 16.0,
        surface_bias: 0.0025,
    }
}
