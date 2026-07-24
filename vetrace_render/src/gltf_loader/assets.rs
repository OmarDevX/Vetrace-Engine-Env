use super::*;

pub(crate) fn ensure_render_assets(engine: &mut Engine) {
    if !engine.contains_resource::<RenderAssets>() {
        engine.insert_resource(RenderAssets::default());
    }
}

pub(crate) fn render_assets_mut(engine: &mut Engine) -> &mut RenderAssets {
    ensure_render_assets(engine);
    engine.get_resource_mut::<RenderAssets>().expect("RenderAssets was just inserted")
}

pub(crate) fn select_scene<'a>(document: &'a gltf::Document, requested: Option<usize>) -> Result<gltf::Scene<'a>> {
    if let Some(index) = requested {
        return document
            .scenes()
            .nth(index)
            .with_context(|| format!("glTF scene index {index} does not exist"));
    }
    document
        .default_scene()
        .or_else(|| document.scenes().next())
        .context("glTF document has no scenes")
}
