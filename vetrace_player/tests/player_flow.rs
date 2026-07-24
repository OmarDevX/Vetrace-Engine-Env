use std::path::PathBuf;

use vetrace_build::{create_vpak, PackageOptions};
use vetrace_player::{PlayerArgs, run};
use vetrace_project::VetraceProject;

fn example_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("lua_runtime_project")
}

#[test]
fn validates_a_project_without_starting_runtime() {
    let args = PlayerArgs {
        project: Some(example_project()),
        validate_only: true,
        ..PlayerArgs::default()
    };
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    run(args, &mut output, &mut diagnostics).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Project validation passed"));
}

#[test]
fn prints_project_information_without_starting_runtime() {
    let args = PlayerArgs {
        project: Some(example_project()),
        print_project_info: true,
        ..PlayerArgs::default()
    };
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    run(args, &mut output, &mut diagnostics).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Project: Lua Runtime Example"));
    assert!(output.contains("Main scene: assets/scenes/main.vscene"));
    assert!(!output.contains("Launching"));
}

#[test]
fn fixed_timestep_override_is_applied_to_the_effective_project() {
    let args = PlayerArgs {
        project: Some(example_project()),
        fixed_dt: Some(0.02),
        print_project_info: true,
        ..PlayerArgs::default()
    };
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    run(args, &mut output, &mut diagnostics).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Fixed timestep: 0.02"));
}

#[test]
fn runs_the_generic_project_headlessly_for_a_bounded_frame_count() {
    let args = PlayerArgs {
        project: Some(example_project()),
        headless: true,
        max_frames: Some(3),
        ..PlayerArgs::default()
    };
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    run(args, &mut output, &mut diagnostics).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("HeadlessServer"));
    assert!(output.contains("stopped after 3 frame(s)"));
}

#[test]
fn runs_a_packaged_project_headlessly() {
    let directory = std::env::temp_dir().join(format!(
        "vetrace-player-package-test-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let package = directory.join("game.vpak");
    let project = VetraceProject::load(example_project()).unwrap();
    create_vpak(&project, &package, PackageOptions::default()).unwrap();

    let args = PlayerArgs {
        package: Some(package),
        headless: true,
        max_frames: Some(2),
        ..PlayerArgs::default()
    };
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    run(args, &mut output, &mut diagnostics).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Launching packaged"));
    assert!(output.contains("stopped after 2 frame(s)"));
    std::fs::remove_dir_all(directory).unwrap();
}
