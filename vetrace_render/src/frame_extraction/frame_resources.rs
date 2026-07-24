use super::*;

pub(super) fn valid_cubemap_handle(
    assets: Option<&RenderAssets>,
    handle: Option<CubemapHandle>,
) -> Option<CubemapHandle> {
    handle.filter(|handle| {
        assets
            .and_then(|assets| assets.cubemaps.get(&handle.0))
            .is_some_and(|cubemap| cubemap.is_valid())
    })
}

pub(super) fn extract_environment(
    engine: &Engine,
    assets: Option<&RenderAssets>,
) -> Option<RenderEnvironment> {
    engine
        .get_resource::<EnvironmentCubemap>()
        .filter(|environment| environment.enabled)
        .map(|environment| RenderEnvironment {
            primary: valid_cubemap_handle(assets, environment.primary),
            secondary: valid_cubemap_handle(assets, environment.secondary),
            transition: environment.transition.clamp(0.0, 1.0),
            intensity: environment.intensity.max(0.0),
            rotation_radians: environment.rotation_radians,
            draw_sky: environment.draw_sky,
            diffuse_ibl: environment.diffuse_ibl,
            specular_ibl: environment.specular_ibl,
        })
        .filter(|environment| environment.primary.is_some() || environment.secondary.is_some())
}

pub(super) fn extract_custom_post_process_passes(
    engine: &Engine,
    post_processing: &PostProcessing,
) -> Vec<CustomPostProcessPass> {
    let mut passes = Vec::new();
    if let Some(ssr) = engine.get_resource::<ScreenSpaceReflections>() {
        if ssr.enabled {
            upsert_custom_post_process_pass(&mut passes, ssr.as_custom_post_process_pass());
        }
    }
    if post_processing.bloom.enabled {
        upsert_custom_post_process_pass(
            &mut passes,
            built_in_bloom_pass(&post_processing.bloom),
        );
    }
    if let Some(pass) = engine.get_resource::<CustomPostProcessPass>() {
        if pass.enabled {
            upsert_custom_post_process_pass(&mut passes, pass.clone());
        }
    }
    if let Some(stack) = engine.get_resource::<CustomPostProcessStack>() {
        for pass in stack.passes.iter().filter(|pass| pass.enabled) {
            upsert_custom_post_process_pass(&mut passes, pass.clone());
        }
    }
    passes.sort_by_key(|pass| pass.order);
    passes
}
