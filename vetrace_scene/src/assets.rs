use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec2;
use vetrace_core::{Engine, Entity};
use vetrace_render::{Material, RenderAssets, TextureAsset, TextureHandle};
#[cfg(feature = "render_2d")]
use vetrace_render::Sprite2D;

#[derive(Clone, Debug, Default)]
pub struct SceneTextureLoadReport {
    pub loaded: usize,
    pub reused: usize,
    pub missing: usize,
}

impl SceneTextureLoadReport {
    pub fn any_work_done(&self) -> bool { self.loaded > 0 || self.reused > 0 || self.missing > 0 }
}

/// Reloads every portable renderer texture reference used by a scene and
/// reconnects it to live process-local `TextureHandle`s.
pub fn load_scene_render_textures(
    engine: &mut Engine,
    entities: &[Entity],
    scene_path: Option<&Path>,
) -> SceneTextureLoadReport {
    let mut report = SceneTextureLoadReport::default();
    let mut cache: HashMap<PathBuf, (TextureHandle, u32, u32)> = HashMap::new();

    load_material_textures(engine, entities, scene_path, &mut cache, &mut report);
    #[cfg(feature = "render_2d")]
    load_sprite_2d_textures(engine, entities, scene_path, &mut cache, &mut report);

    report
}

/// Backward-compatible material-only entry point. New scene loading should call
/// `load_scene_render_textures` so optional renderer families share one cache.
pub fn load_scene_material_textures(
    engine: &mut Engine,
    entities: &[Entity],
    scene_path: Option<&Path>,
) -> SceneTextureLoadReport {
    load_scene_render_textures(engine, entities, scene_path)
}

fn load_material_textures(
    engine: &mut Engine,
    entities: &[Entity],
    scene_path: Option<&Path>,
    cache: &mut HashMap<PathBuf, (TextureHandle, u32, u32)>,
    report: &mut SceneTextureLoadReport,
) {
    for entity in entities.iter().copied() {
        let Some(path_text) = engine
            .raw_world()
            .get::<Material>(entity)
            .and_then(|material| material.base_color_texture_path.clone())
        else {
            continue;
        };

        let texture_path = resolve_scene_texture_path(&path_text, scene_path);
        let Some((handle, width, height)) = load_cached_texture(engine, texture_path, cache, report) else {
            continue;
        };

        if let Some(material) = engine.raw_world_mut().get_mut::<Material>(entity) {
            material.base_color_texture = Some(handle);
            if (material.uv_scale.x - material.uv_scale.y).abs() <= 0.0001 {
                material.uv_scale = aspect_correct_uv_scale(material.uv_scale.y, width, height);
            }
        }
    }
}

#[cfg(feature = "render_2d")]
fn load_sprite_2d_textures(
    engine: &mut Engine,
    entities: &[Entity],
    scene_path: Option<&Path>,
    cache: &mut HashMap<PathBuf, (TextureHandle, u32, u32)>,
    report: &mut SceneTextureLoadReport,
) {
    for entity in entities.iter().copied() {
        let Some(path_text) = engine
            .raw_world()
            .get::<Sprite2D>(entity)
            .and_then(|sprite| sprite.texture_path.clone())
        else {
            continue;
        };

        if let Some(sprite) = engine.raw_world_mut().get_mut::<Sprite2D>(entity) {
            sprite.texture = None;
        }
        let texture_path = resolve_scene_texture_path(&path_text, scene_path);
        let Some((handle, _, _)) = load_cached_texture(engine, texture_path, cache, report) else {
            continue;
        };
        if let Some(sprite) = engine.raw_world_mut().get_mut::<Sprite2D>(entity) {
            sprite.texture = Some(handle);
        }
    }
}

fn load_cached_texture(
    engine: &mut Engine,
    texture_path: PathBuf,
    cache: &mut HashMap<PathBuf, (TextureHandle, u32, u32)>,
    report: &mut SceneTextureLoadReport,
) -> Option<(TextureHandle, u32, u32)> {
    if let Some(cached) = cache.get(&texture_path).copied() {
        report.reused = report.reused.saturating_add(1);
        return Some(cached);
    }

    let (texture, width, height) = match load_texture_asset_from_path(&texture_path) {
        Ok(texture) => texture,
        Err(_) => {
            report.missing = report.missing.saturating_add(1);
            return None;
        }
    };
    if !engine.contains_resource::<RenderAssets>() {
        engine.insert_resource(RenderAssets::default());
    }
    let handle = engine
        .get_resource_mut::<RenderAssets>()
        .expect("RenderAssets inserted before scene texture reload")
        .insert_texture(texture);
    let loaded = (handle, width, height);
    cache.insert(texture_path, loaded);
    report.loaded = report.loaded.saturating_add(1);
    Some(loaded)
}

fn load_texture_asset_from_path(path: &Path) -> anyhow::Result<(TextureAsset, u32, u32)> {
    let image = image::open(path).map_err(|err| anyhow::anyhow!("could not decode image: {err}"))?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        anyhow::bail!("image has zero size");
    }
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("scene_texture").to_string();
    Ok((TextureAsset { name, width, height, rgba8: rgba.into_raw(), revision: 0 }, width, height))
}

fn aspect_correct_uv_scale(tile_scale: f32, width: u32, height: u32) -> Vec2 {
    let tile_scale = tile_scale.max(0.0001);
    let aspect = (width.max(1) as f32 / height.max(1) as f32).max(0.0001);
    Vec2::new(tile_scale / aspect, tile_scale)
}

fn resolve_scene_texture_path(path_text: &str, scene_path: Option<&Path>) -> PathBuf {
    let expanded = expand_user_path(path_text.trim());
    let scene_dir = scene_path.and_then(Path::parent);

    if expanded.is_absolute() {
        if let (Some(scene_dir), Some(file_name)) = (scene_dir, expanded.file_name()) {
            let asset_candidate = scene_dir.join("assets").join("textures").join(file_name);
            if asset_candidate.exists() {
                return asset_candidate;
            }
            let sibling_candidate = scene_dir.join(file_name);
            if sibling_candidate.exists() {
                return sibling_candidate;
            }
            return asset_candidate;
        }
        if let Some(file_name) = expanded.file_name() {
            return PathBuf::from(file_name);
        }
        return expanded;
    }

    if let Some(scene_dir) = scene_dir {
        // Project asset paths are stored as `assets/...`. Walk upward from the
        // scene directory so a scene in `assets/scenes/` resolves them against
        // the project root rather than `assets/scenes/assets/...`.
        if expanded.starts_with("assets") {
            for ancestor in scene_dir.ancestors() {
                let candidate = ancestor.join(&expanded);
                if candidate.exists() {
                    return candidate;
                }
            }
        }
        let candidate = scene_dir.join(&expanded);
        if candidate.exists() {
            return candidate;
        }
        return candidate;
    }

    expanded
}

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}
