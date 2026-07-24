use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use vetrace_build::{
    build_project, create_template_bundle, create_vpak, inspect_vpak, mount_vpak,
    package_macos_app, package_portable_zip, write_player_template_metadata,
    BuildAssetPreflight, BuildRequest, DistributionArtifact, ExportPreset,
    InstalledPlayerTemplate, PlayerTemplateCatalog, PlayerTemplateCatalogEntry,
    PlayerTemplateManager, PlayerTemplateMetadata, PackageManifest, PackageOptions,
    TEMPLATE_CATALOG_FORMAT_VERSION, VPAK_MANIFEST_FILE,
};
use vetrace_project::VetraceProject;

fn example_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("lua_runtime_project")
}

fn temporary_directory(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "vetrace-build-test-{label}-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_directory(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}


fn rewrite_package_manifest(
    package: &Path,
    edit: impl FnOnce(&mut PackageManifest),
) {
    let input = fs::File::open(package).unwrap();
    let mut archive = zip::ZipArchive::new(input).unwrap();
    let temporary = package.with_extension("rewritten.vpak");
    let output = fs::File::create(&temporary).unwrap();
    let mut writer = zip::ZipWriter::new(output);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let mut edit = Some(edit);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_owned();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name == VPAK_MANIFEST_FILE {
            let mut manifest: PackageManifest = serde_json::from_slice(&bytes).unwrap();
            edit.take().unwrap()(&mut manifest);
            bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        }
        writer.start_file(name, options.clone()).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
    fs::remove_file(package).unwrap();
    fs::rename(temporary, package).unwrap();
}

#[test]
fn package_round_trip_preserves_a_runnable_project() {
    let root = temporary_directory("package");
    let project_root = root.join("project");
    copy_directory(&example_project(), &project_root);
    let project = VetraceProject::load(&project_root).unwrap();
    let package = root.join("game.vpak");

    let created = create_vpak(&project, &package, PackageOptions::default()).unwrap();
    assert!(created.entries.iter().any(|entry| entry.path == "project.vetrace.toml"));
    assert!(created.entries.iter().any(|entry| entry.path.ends_with("main.vscene")));

    let inspected = inspect_vpak(&package).unwrap();
    assert_eq!(inspected.project_id, project.manifest().project.id);

    let mount = mount_vpak(&package).unwrap();
    assert_eq!(mount.project().manifest().project.name, project.manifest().project.name);
    assert!(mount.project().main_scene_path().is_file());
    drop(mount);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_copies_a_prebuilt_player_without_invoking_cargo() {
    let root = temporary_directory("export");
    let project_root = root.join("project");
    copy_directory(&example_project(), &project_root);
    let project = VetraceProject::load(&project_root).unwrap();
    let template = root.join(if cfg!(windows) { "vetrace-player.exe" } else { "vetrace-player" });
    fs::write(&template, b"prebuilt-player-template").unwrap();
    let metadata = PlayerTemplateMetadata::compiled_for_current_player().unwrap();
    write_player_template_metadata(&template, &metadata).unwrap();

    let report = build_project(&BuildRequest {
        project,
        preset: ExportPreset::default(),
        player_template: template,
        asset_preflight: BuildAssetPreflight::Refresh,
    }).unwrap();

    assert!(report.executable.is_file());
    assert!(report.package.is_file());
    assert!(report.output_directory.join("build-report.json").is_file());
    assert_eq!(fs::read(&report.executable).unwrap(), b"prebuilt-player-template");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsafe_output_directories_are_rejected() {
    let mut preset = ExportPreset::default();
    preset.output_directory = vetrace_project::ProjectPath::new("assets/export").unwrap();
    assert!(preset.validate().is_err());
}

#[test]
fn export_config_round_trips() {
    let root = temporary_directory("config");
    let project_root = root.join("project");
    copy_directory(&example_project(), &project_root);
    let project = VetraceProject::load(&project_root).unwrap();
    let config = vetrace_build::ExportConfig::default();
    config.save(&project).unwrap();
    let loaded = vetrace_build::ExportConfig::load_or_default(&project).unwrap();
    assert_eq!(loaded, config);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn package_mount_is_removed_when_dropped() {
    let root = temporary_directory("cleanup");
    let project = VetraceProject::load(example_project()).unwrap();
    let package = root.join("game.vpak");
    create_vpak(&project, &package, PackageOptions::default()).unwrap();
    let mount = mount_vpak(&package).unwrap();
    let mounted_root = mount.root().to_path_buf();
    assert!(mounted_root.is_dir());
    drop(mount);
    assert!(!mounted_root.exists());
    fs::remove_dir_all(root).unwrap();
}


#[test]
fn package_metadata_must_match_the_embedded_project() {
    let root = temporary_directory("identity");
    let project = VetraceProject::load(example_project()).unwrap();
    let package = root.join("game.vpak");
    create_vpak(&project, &package, PackageOptions::default()).unwrap();
    rewrite_package_manifest(&package, |manifest| {
        manifest.project_name.push_str(" tampered");
    });
    let error = mount_vpak(&package).err().expect("tampered package must fail");
    assert!(error.to_string().contains("metadata does not match"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn replacing_an_export_removes_stale_previous_contents() {
    let root = temporary_directory("replace");
    let project_root = root.join("project");
    copy_directory(&example_project(), &project_root);
    let template = root.join(if cfg!(windows) { "vetrace-player.exe" } else { "vetrace-player" });
    fs::write(&template, b"player-v1").unwrap();
    let metadata = PlayerTemplateMetadata::compiled_for_current_player().unwrap();
    write_player_template_metadata(&template, &metadata).unwrap();
    let first = build_project(&BuildRequest {
        project: VetraceProject::load(&project_root).unwrap(),
        preset: ExportPreset::default(),
        player_template: template.clone(),
        asset_preflight: BuildAssetPreflight::Refresh,
    }).unwrap();
    fs::write(first.output_directory.join("stale.txt"), b"stale").unwrap();
    fs::write(&template, b"player-v2").unwrap();
    let second = build_project(&BuildRequest {
        project: VetraceProject::load(&project_root).unwrap(),
        preset: ExportPreset::default(),
        player_template: template,
        asset_preflight: BuildAssetPreflight::Refresh,
    }).unwrap();
    assert_eq!(fs::read(second.executable).unwrap(), b"player-v2");
    assert!(!second.output_directory.join("stale.txt").exists());
    fs::remove_dir_all(root).unwrap();
}


#[test]
fn template_bundle_install_reopen_and_remove_round_trip() {
    let root = temporary_directory("template-manager");
    let binary = root.join(if cfg!(windows) { "vetrace-player.exe" } else { "vetrace-player" });
    fs::write(&binary, b"managed-player-template").unwrap();
    let metadata = PlayerTemplateMetadata::compiled_for_current_player().unwrap();
    write_player_template_metadata(&binary, &metadata).unwrap();
    let bundle = root.join("standard.vtemplate");
    create_template_bundle("standard host", &binary, &bundle).unwrap();

    let manager_root = root.join("installed");
    let installed: InstalledPlayerTemplate = PlayerTemplateManager::open(&manager_root)
        .unwrap()
        .install_archive(&bundle)
        .unwrap();
    assert_eq!(installed.id, "standard-host");
    assert_eq!(fs::read(&installed.binary).unwrap(), b"managed-player-template");
    assert!(installed.metadata.is_file());

    let mut reopened = PlayerTemplateManager::open(&manager_root).unwrap();
    assert!(reopened.find(metadata.target, &metadata.engine_version).is_some());
    assert!(reopened.remove("standard-host").unwrap());
    assert!(reopened.templates().is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn template_manager_rejects_incompatible_metadata_shape() {
    let root = temporary_directory("template-invalid");
    let binary = root.join("player");
    fs::write(&binary, b"player").unwrap();
    let mut metadata = PlayerTemplateMetadata::compiled_for_current_player().unwrap();
    metadata.format_version += 1;
    let mut manager = PlayerTemplateManager::open(root.join("installed")).unwrap();
    assert!(manager.install_binary("invalid", &binary, &metadata).is_err());
    assert!(manager.templates().is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn template_catalog_validates_security_and_round_trips() {
    let root = temporary_directory("template-catalog");
    let metadata = PlayerTemplateMetadata::compiled_for_current_player().unwrap();
    let catalog = PlayerTemplateCatalog {
        format_version: TEMPLATE_CATALOG_FORMAT_VERSION,
        generated_unix_ms: 1,
        templates: vec![PlayerTemplateCatalogEntry {
            id: "standard".into(),
            engine_version: metadata.engine_version,
            target: metadata.target,
            url: "https://templates.example.invalid/standard.vtemplate".into(),
            blake3: "0".repeat(64),
            bytes: Some(128),
        }],
    };
    let path = root.join("catalog.json");
    catalog.save(&path).unwrap();
    assert_eq!(PlayerTemplateCatalog::load(&path).unwrap(), catalog);

    let mut insecure = catalog;
    insecure.templates[0].url = "http://templates.example.invalid/template.vtemplate".into();
    assert!(insecure.validate().is_err());
    insecure.templates[0].url = "http://127.0.0.1/template.vtemplate".into();
    assert!(insecure.validate().is_ok());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn distribution_packaging_creates_portable_and_macos_layouts() {
    let root = temporary_directory("distribution");
    let build = root.join("build");
    fs::create_dir_all(build.join("licenses")).unwrap();
    let executable = build.join("ExampleGame");
    fs::write(&executable, b"player").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).unwrap();
    }
    fs::write(build.join("game.vpak"), b"package").unwrap();
    fs::write(build.join("licenses/LICENSE.txt"), b"license").unwrap();

    let archive = root.join("ExampleGame.zip");
    assert_eq!(
        package_portable_zip(&build, &archive).unwrap(),
        DistributionArtifact::PortableArchive(archive.clone()),
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
    assert!(zip.by_name("ExampleGame").is_ok());
    assert!(zip.by_name("game.vpak").is_ok());
    assert!(zip.by_name("licenses/LICENSE.txt").is_ok());

    let app = root.join("ExampleGame.app");
    assert_eq!(
        package_macos_app(&build, "Example & Game", &app).unwrap(),
        DistributionArtifact::MacApplication(app.clone()),
    );
    assert!(app.join("Contents/MacOS/ExampleGame").is_file());
    let plist = fs::read_to_string(app.join("Contents/Info.plist")).unwrap();
    assert!(plist.contains("Example &amp; Game"));
    assert!(plist.contains("<string>ExampleGame</string>"));
    fs::remove_dir_all(root).unwrap();
}
