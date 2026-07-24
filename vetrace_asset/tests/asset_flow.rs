use std::fs;

use vetrace_asset::{AssetKind, AssetManager, AssetStatus};
use vetrace_project::{ProjectManifest, VetraceProject};

fn temporary_project(name: &str) -> (std::path::PathBuf, VetraceProject) {
    let root = std::env::temp_dir().join(format!("vetrace-asset-{name}-{}", uuid::Uuid::new_v4()));
    let project = VetraceProject::create(&root, ProjectManifest::new(name, "0.1.0")).unwrap();
    (root, project)
}

#[test]
fn discovers_imports_and_persists_stable_ids() {
    let (root, project) = temporary_project("stable");
    let source = project.paths().scripts().join("player.lua");
    fs::write(&source, "return {}\n").unwrap();

    let mut assets = AssetManager::open(&project).unwrap();
    let report = assets.refresh().unwrap();
    assert_eq!(report.added, 1);
    assert_eq!(report.imported, 1);
    let record = assets.database().records.values().next().unwrap().clone();
    assert_eq!(record.kind, AssetKind::Script);
    assert_eq!(record.status, AssetStatus::Ready);

    fs::write(&source, "return { changed = true }\n").unwrap();
    assets.refresh().unwrap();
    assert_eq!(assets.database().records.values().next().unwrap().id, record.id);

    let renamed = project.paths().scripts().join("renamed.lua");
    fs::rename(&source, &renamed).unwrap();
    assets.refresh().unwrap();
    let renamed_record = assets.database().records.values()
        .find(|record| record.status != AssetStatus::Missing)
        .unwrap();
    assert_eq!(renamed_record.id, record.id);
    assert!(renamed_record.source.as_str().ends_with("renamed.lua"));
    drop(assets);
    let reopened = AssetManager::open(&project).unwrap();
    assert_eq!(reopened.database().records.values()
        .find(|record| record.status != AssetStatus::Missing)
        .unwrap().id, record.id);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn reports_missing_scene_dependencies() {
    let (root, project) = temporary_project("dependencies");
    let scene = project.paths().scenes().join("main.vscene");
    fs::write(&scene, r#"{"script":"assets/scripts/missing.lua"}"#).unwrap();
    let mut assets = AssetManager::open(&project).unwrap();
    assets.refresh().unwrap();
    assert!(assets.database().diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "asset.dependency_missing"
            && diagnostic.dependency.as_ref().is_some_and(|path| path.as_str() == "assets/scripts/missing.lua")
    }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cache_pruning_removes_only_orphans() {
    let (root, project) = temporary_project("cache");
    fs::write(project.paths().scripts().join("player.lua"), "return {}\n").unwrap();
    let orphan = project.paths().imported().join("not-an-asset-id");
    fs::create_dir_all(&orphan).unwrap();
    fs::write(orphan.join("orphan.bin"), b"orphan").unwrap();
    let mut assets = AssetManager::open(&project).unwrap();
    assets.refresh().unwrap();
    assert_eq!(assets.prune_cache().unwrap(), 1);
    assert!(!orphan.exists());
    assert_eq!(assets.database().records.values().filter(|record| record.status == AssetStatus::Ready).count(), 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn custom_importer_requires_no_database_or_studio_changes() {
    use std::fs;
    use vetrace_asset::{
        AssetImporter, AssetKind, AssetResult, GenericCopyImporter, ImportContext, ImportOutput,
        ImporterRegistry,
    };

    struct CustomImporter(GenericCopyImporter);
    impl AssetImporter for CustomImporter {
        fn id(&self) -> &str { "test.custom" }
        fn version(&self) -> u32 { 7 }
        fn kind(&self) -> AssetKind { AssetKind::Custom("Custom Test".into()) }
        fn extensions(&self) -> &[String] { self.0.extensions() }
        fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput> {
            self.0.import(context)
        }
    }

    let (root, project) = temporary_project("custom");
    fs::write(project.paths().assets().join("sample.foo"), b"custom").unwrap();
    let mut registry = ImporterRegistry::new();
    registry.register(CustomImporter(GenericCopyImporter::new(
        "test.custom-copy",
        1,
        AssetKind::Custom("Custom Test".into()),
        ["foo"],
    )));
    let mut assets = AssetManager::with_registry(&project, registry).unwrap();
    assets.refresh().unwrap();
    let record = assets.database().records.values().next().unwrap();
    assert_eq!(record.kind, AssetKind::Custom("Custom Test".into()));
    assert_eq!(record.importer.as_ref().unwrap().id, "test.custom");
    assert_eq!(record.importer.as_ref().unwrap().version, 7);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn replacing_an_importer_replaces_its_extension_ownership() {
    use std::path::Path;
    use vetrace_asset::{AssetKind, GenericCopyImporter, ImporterRegistry};

    let mut registry = ImporterRegistry::new();
    registry.register(GenericCopyImporter::new(
        "test.replaceable",
        1,
        AssetKind::Data,
        ["old", "kept"],
    ));
    registry.register(GenericCopyImporter::new(
        "test.replaceable",
        2,
        AssetKind::Data,
        ["kept", "new"],
    ));

    assert!(registry.importer_for_path(Path::new("file.old")).is_none());
    assert_eq!(registry.importer_for_path(Path::new("file.kept")).unwrap().version(), 2);
    assert_eq!(registry.importer_for_path(Path::new("file.new")).unwrap().version(), 2);
}

#[test]
fn external_imports_are_routed_by_registered_asset_kind() {
    let (root, project) = temporary_project("external-import");
    let external_root = std::env::temp_dir().join(format!(
        "vetrace-external-assets-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&external_root).unwrap();
    let external = external_root.join("surface.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([40, 80, 120, 255]))
        .save(&external)
        .unwrap();

    let mut assets = AssetManager::open(&project).unwrap();
    let imported = assets.import_external_files(&[external.clone()], None).unwrap();
    assert_eq!(imported.len(), 1);
    assert!(imported[0].destination.as_str().starts_with("assets/textures/"));
    assert!(project.paths().resolve(&imported[0].destination).exists());
    assert_eq!(assets.database().records.values().filter(|record| record.kind == AssetKind::Texture).count(), 1);

    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(external_root);
}


fn pcm16_mono_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_bytes = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples { bytes.extend_from_slice(&sample.to_le_bytes()); }
    bytes
}

#[test]
fn production_importers_generate_runtime_outputs_and_thumbnails() {
    let (root, project) = temporary_project("production-importers");
    let texture = project.paths().textures().join("surface.png");
    image::RgbaImage::from_pixel(4, 2, image::Rgba([255, 128, 32, 255]))
        .save(&texture)
        .unwrap();
    let audio = project.paths().audio().join("tone.wav");
    fs::write(&audio, pcm16_mono_wav(&[0, 8_000, 16_000, 0, -16_000, -8_000], 48_000)).unwrap();
    let shader = project.paths().assets().join("surface.wgsl");
    fs::write(&shader, "@compute @workgroup_size(1) fn main() {}\n").unwrap();

    let mut assets = AssetManager::open(&project).unwrap();
    let report = assets.refresh().unwrap();
    assert_eq!(report.failed, 0);
    for source in ["assets/textures/surface.png", "assets/audio/tone.wav"] {
        let record = assets.database().records.values().find(|record| record.source.as_str() == source).unwrap();
        assert_eq!(record.status, AssetStatus::Ready);
        assert!(record.outputs.iter().any(|path| path.as_str().ends_with("thumbnail.png")));
        for output in &record.outputs {
            assert!(project.paths().resolve(output).is_file(), "missing imported output {output}");
        }
        let metadata = vetrace_asset::AssetDatabase::metadata_path(record.id).unwrap();
        let metadata: serde_json::Value = serde_json::from_slice(&fs::read(project.paths().resolve(&metadata)).unwrap()).unwrap();
        assert!(metadata.get("metadata").is_some());
    }
    let shader_record = assets.database().records.values()
        .find(|record| record.source.as_str() == "assets/surface.wgsl")
        .unwrap();
    assert_eq!(shader_record.status, AssetStatus::Ready);
    assert!(shader_record.outputs.iter().any(|path| path.as_str().ends_with("shader.wgsl")));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn invalid_shader_isolated_as_failed_asset() {
    let (root, project) = temporary_project("invalid-shader");
    fs::write(project.paths().assets().join("broken.wgsl"), "this is not wgsl").unwrap();
    let mut assets = AssetManager::open(&project).unwrap();
    let report = assets.refresh().unwrap();
    assert_eq!(report.failed, 1);
    let record = assets.database().records.values().next().unwrap();
    assert_eq!(record.status, AssetStatus::Failed);
    assert!(record.last_error.as_deref().is_some_and(|error| error.contains("WGSL")));
    let _ = fs::remove_dir_all(root);
}
