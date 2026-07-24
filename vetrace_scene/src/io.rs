use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::document::{SceneDocument, SceneResources};
use crate::legacy::LegacyPrefabDocument;
use crate::SceneNode;

pub fn load_scene_file(path: impl AsRef<Path>) -> Result<SceneDocument> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scene JSON {}", path.display()))?;
    let document = parse_scene_text(&text)
        .with_context(|| format!("failed to parse scene JSON {}", path.display()))?;
    document.validate()?;
    Ok(document)
}

pub fn save_scene_file(path: impl AsRef<Path>, document: &SceneDocument) -> Result<()> {
    let path = path.as_ref();
    document.validate()?;
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create scene directory {}", parent.display()))?;
    }
    let contents = document.to_pretty_json()?;
    write_scene_atomic(path, contents.as_bytes())
}


fn write_scene_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("scene.vscene");
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    let result = (|| -> Result<()> {
        let mut file = std::fs::File::create(&temporary)
            .with_context(|| format!("failed to create temporary scene {}", temporary.display()))?;
        file.write_all(contents)
            .with_context(|| format!("failed to write temporary scene {}", temporary.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to flush temporary scene {}", temporary.display()))?;
        drop(file);
        #[cfg(windows)]
        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("failed to replace scene {}", path.display()))?;
        }
        std::fs::rename(&temporary, path)
            .with_context(|| format!("failed to replace scene {}", path.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub fn parse_scene_text(text: &str) -> Result<SceneDocument> {
    let value: Value = serde_json::from_str(text)?;
    if value.get("roots").is_some() {
        return Ok(serde_json::from_value(value)?);
    }

    let legacy: LegacyPrefabDocument = serde_json::from_value(value)?;
    let roots = legacy.objects.into_iter().map(SceneNode::from_legacy_object).collect();
    Ok(SceneDocument { version: legacy.version, name: legacy.name, roots, resources: SceneResources::default() })
}


/// Migrates a readable legacy scene to the current scene version in place.
/// The original file is preserved as `<name>.vscene.bak`.
pub fn migrate_scene_file(path: impl AsRef<Path>) -> Result<bool> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scene JSON {}", path.display()))?;
    let mut document = parse_scene_text(&text)
        .with_context(|| format!("failed to parse scene JSON {}", path.display()))?;
    if document.version == crate::SCENE_VERSION {
        return Ok(false);
    }
    if document.version > crate::SCENE_VERSION {
        anyhow::bail!(
            "scene version {} is newer than supported version {}",
            document.version,
            crate::SCENE_VERSION
        );
    }
    let backup = path.with_extension(format!(
        "{}.bak",
        path.extension().and_then(|extension| extension.to_str()).unwrap_or("vscene")
    ));
    std::fs::copy(path, &backup)
        .with_context(|| format!("failed to back up scene {}", path.display()))?;
    document.version = crate::SCENE_VERSION;
    save_scene_file(path, &document)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SceneDocument, SCENE_VERSION};

    fn temporary_scene(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "vetrace-scene-{name}-{}-{}.vscene",
            std::process::id(),
            uuid::Uuid::new_v4(),
        ))
    }

    #[test]
    fn save_replaces_scene_atomically() {
        let path = temporary_scene("atomic");
        let first = SceneDocument::new("First");
        save_scene_file(&path, &first).unwrap();
        let second = SceneDocument::new("Second");
        save_scene_file(&path, &second).unwrap();
        assert_eq!(load_scene_file(&path).unwrap().name, "Second");
        let temporary = path.with_file_name(format!(
            ".{}.tmp",
            path.file_name().unwrap().to_string_lossy(),
        ));
        assert!(!temporary.exists());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migration_backs_up_and_upgrades_older_scene() {
        let path = temporary_scene("migration");
        let mut document = SceneDocument::new("Legacy");
        document.version = SCENE_VERSION.saturating_sub(1);
        std::fs::write(&path, document.to_pretty_json().unwrap()).unwrap();

        assert!(migrate_scene_file(&path).unwrap());
        assert_eq!(load_scene_file(&path).unwrap().version, SCENE_VERSION);
        let backup = path.with_extension("vscene.bak");
        assert!(backup.is_file());
        let backed_up: SceneDocument = serde_json::from_str(&std::fs::read_to_string(&backup).unwrap()).unwrap();
        assert_eq!(backed_up.version, SCENE_VERSION.saturating_sub(1));
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(backup);
    }

    #[test]
    fn migration_rejects_newer_scene_without_overwriting() {
        let path = temporary_scene("newer");
        let mut document = SceneDocument::new("Future");
        document.version = SCENE_VERSION.saturating_add(1);
        let source = document.to_pretty_json().unwrap();
        std::fs::write(&path, &source).unwrap();

        assert!(migrate_scene_file(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
        assert!(!path.with_extension("vscene.bak").exists());
        let _ = std::fs::remove_file(path);
    }
}
