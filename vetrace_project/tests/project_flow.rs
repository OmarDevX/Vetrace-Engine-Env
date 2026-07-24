use std::fs;
use std::path::PathBuf;

use vetrace_project::{
    discover_projects, find_project_root, migrate_project, InputAction, ProjectManifest, ProjectPath,
    VetraceProject, CURRENT_PROJECT_FORMAT_VERSION, PROJECT_MANIFEST_FILE,
};

fn temporary_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "vetrace_project_test_{name}_{}_{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ))
}

#[test]
fn creates_round_trips_and_discovers_project() {
    let root = temporary_root("round_trip");
    let mut manifest = ProjectManifest::new("Lua Test", "0.28.0");
    manifest.input.insert(
        "jump",
        InputAction {
            keys: vec!["Space".to_owned()],
            ..InputAction::default()
        },
    );

    let _project = VetraceProject::create(&root, manifest.clone()).unwrap();
    assert!(root.join(PROJECT_MANIFEST_FILE).is_file());
    for directory in [
        "assets/scenes",
        "assets/scripts",
        "assets/models",
        "assets/textures",
        "assets/audio",
        "assets/fonts",
        ".vetrace/imported",
        ".vetrace/cache",
        ".vetrace/editor",
        "builds",
    ] {
        assert!(root.join(directory).is_dir(), "missing standard directory {directory}");
    }

    fs::create_dir_all(root.join("assets/scenes")).unwrap();
    fs::write(root.join("assets/scenes/main.vscene"), "{}").unwrap();

    let loaded = VetraceProject::load(&root).unwrap();
    assert_eq!(loaded.manifest(), &manifest);
    assert!(loaded.validate_files().is_valid());

    let nested = root.join("assets/scripts/player");
    fs::create_dir_all(&nested).unwrap();
    assert_eq!(find_project_root(&nested).unwrap(), root);
    assert_eq!(VetraceProject::discover(&nested).unwrap().manifest(), &manifest);

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn reports_missing_required_files() {
    let root = temporary_root("missing_files");
    let project = VetraceProject::create_new(&root, "Missing", "0.28.0").unwrap();
    let report = project.validate_files();
    assert!(!report.is_valid());
    assert!(report.errors().any(|issue| issue.code == "main_scene_missing"));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn recursive_discovery_skips_nested_contents_of_projects() {
    let root = temporary_root("discovery");
    let one = root.join("one");
    let two = root.join("group/two");
    VetraceProject::create_new(&one, "One", "0.28.0").unwrap();
    VetraceProject::create_new(&two, "Two", "0.28.0").unwrap();

    let projects = discover_projects(&root, 4).unwrap();
    assert_eq!(projects, vec![two, one]);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn path_conversion_cannot_escape_root() {
    let root = temporary_root("paths");
    let project = VetraceProject::create_new(&root, "Paths", "0.28.0").unwrap();
    let asset = project
        .paths()
        .to_project_path(root.join("assets/models/player.glb"))
        .unwrap();
    assert_eq!(asset, ProjectPath::new("assets/models/player.glb").unwrap());
    assert!(project.paths().to_project_path(root.join("../secret.txt")).is_err());
    fs::remove_dir_all(&root).unwrap();
}


#[test]
fn manifest_toml_round_trip_preserves_settings() {
    let mut manifest = ProjectManifest::new("Round Trip", "0.28.0");
    manifest.runtime.autoload_scripts = vec![ProjectPath::new("assets/scripts/game.lua").unwrap()];
    manifest.application.cursor_grab = false;
    manifest.application.cursor_visible = true;
    manifest.input.insert(
        "move_forward",
        InputAction {
            keys: vec!["W".to_owned(), "ArrowUp".to_owned()],
            ..InputAction::default()
        },
    );

    let encoded = toml::to_string_pretty(&manifest).unwrap();
    let decoded: ProjectManifest = toml::from_str(&encoded).unwrap();
    assert_eq!(decoded, manifest);
}

#[test]
fn unsafe_manifest_paths_fail_during_parse() {
    let source = r#"
format_version = 1

[runtime]
main_scene = "../outside.vscene"
"#;
    assert!(toml::from_str::<ProjectManifest>(source).is_err());
}

#[test]
fn unchecked_load_allows_editor_to_report_invalid_manifest() {
    let root = temporary_root("unchecked");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join(PROJECT_MANIFEST_FILE),
        r#"
format_version = 999

[project]
name = "Broken"
version = "0.1.0"
engine_version = "0.28.0"
"#,
    )
    .unwrap();

    assert!(VetraceProject::load(&root).is_err());
    let project = VetraceProject::load_unchecked(&root).unwrap();
    let report = project.validate_manifest();
    assert!(report.errors().any(|issue| issue.code == "format_version"));
    assert!(report.errors().any(|issue| issue.code == "project_id_nil"));
    fs::remove_dir_all(&root).unwrap();
}


#[test]
fn migrates_prototype_manifest_with_backup() {
    let root = temporary_root("migration");
    let manifest = ProjectManifest::new("Migrated", env!("CARGO_PKG_VERSION"));
    let mut value = toml::Value::try_from(&manifest).unwrap();
    let table = value.as_table_mut().unwrap();
    table.insert("format_version".to_owned(), toml::Value::Integer(0));
    let features = table.remove("features").unwrap().as_table().unwrap().clone();
    let runtime = table.get_mut("runtime").unwrap().as_table_mut().unwrap();
    for (name, value) in features {
        runtime.insert(name, value);
    }
    fs::create_dir_all(&root).unwrap();
    let manifest_path = root.join(PROJECT_MANIFEST_FILE);
    fs::write(&manifest_path, toml::to_string_pretty(&value).unwrap()).unwrap();

    let report = migrate_project(&root).unwrap();
    assert_eq!(report.from_version, 0);
    assert_eq!(report.to_version, CURRENT_PROJECT_FORMAT_VERSION);
    assert!(report.changed());
    assert!(report.backup.as_ref().is_some_and(|path| path.is_file()));
    assert!(report.changes.iter().any(|change| change.contains("runtime.physics")));
    let loaded = VetraceProject::load(&root).unwrap();
    assert_eq!(loaded.manifest().format_version, CURRENT_PROJECT_FORMAT_VERSION);
    assert_eq!(loaded.manifest().project.name, "Migrated");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn current_manifest_migration_is_a_noop() {
    let root = temporary_root("migration_noop");
    VetraceProject::create_new(&root, "Current", env!("CARGO_PKG_VERSION")).unwrap();
    let report = migrate_project(&root).unwrap();
    assert!(!report.changed());
    assert!(report.backup.is_none());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn migration_rejects_newer_manifest_without_backup() {
    let root = temporary_root("migration_newer");
    let manifest = ProjectManifest::new("Future", env!("CARGO_PKG_VERSION"));
    let mut value = toml::Value::try_from(&manifest).unwrap();
    value.as_table_mut().unwrap().insert(
        "format_version".to_owned(),
        toml::Value::Integer((CURRENT_PROJECT_FORMAT_VERSION + 1) as i64),
    );
    fs::create_dir_all(&root).unwrap();
    let path = root.join(PROJECT_MANIFEST_FILE);
    let source = toml::to_string_pretty(&value).unwrap();
    fs::write(&path, &source).unwrap();

    assert!(migrate_project(&root).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), source);
    assert!(!path.with_extension("toml.bak").exists());
    fs::remove_dir_all(&root).unwrap();
}
