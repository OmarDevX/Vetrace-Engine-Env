use std::path::PathBuf;
use std::process::ExitCode;

use vetrace_build::{
    build_project, find_player_template, BuildAssetPreflight, BuildRequest, ExportConfig,
};
use vetrace_project::VetraceProject;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mut project = PathBuf::from(".");
    let mut preset_name: Option<String> = None;
    let mut player_template: Option<PathBuf> = None;
    let mut positional = false;
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        let text = argument.to_string_lossy();
        match text.as_ref() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-p" | "--project" => {
                project = PathBuf::from(args.next().ok_or("--project requires a path")?);
            }
            "--preset" => {
                preset_name = Some(
                    args.next().ok_or("--preset requires a name")?
                        .to_string_lossy().into_owned(),
                );
            }
            "--player-template" => {
                player_template = Some(PathBuf::from(
                    args.next().ok_or("--player-template requires a path")?,
                ));
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option '{value}'"));
            }
            _ => {
                if positional { return Err("only one positional project path is allowed".to_owned()); }
                positional = true;
                project = PathBuf::from(argument);
            }
        }
    }

    let project = VetraceProject::discover(&project)
        .or_else(|_| VetraceProject::load(&project))
        .map_err(|error| error.to_string())?;
    let config = ExportConfig::load_or_default(&project).map_err(|error| error.to_string())?;
    let preset_name = preset_name.unwrap_or_else(|| config.default_preset.clone());
    let preset = config.preset(&preset_name)
        .cloned()
        .ok_or_else(|| format!("export preset '{preset_name}' was not found"))?;
    let player_template = player_template
        .filter(|path| path.is_file())
        .or_else(find_player_template)
        .ok_or_else(|| {
            "no prebuilt player template found; use --player-template or VETRACE_PLAYER_TEMPLATE"
                .to_owned()
        })?;

    let report = build_project(&BuildRequest {
        project,
        preset,
        player_template,
        asset_preflight: BuildAssetPreflight::Refresh,
    })
        .map_err(|error| error.to_string())?;
    println!("Export complete");
    println!("Output: {}", report.output_directory.display());
    println!("Executable: {}", report.executable.display());
    println!("Package: {}", report.package.display());
    println!("Package entries: {}", report.package_entries);
    println!("Package BLAKE3: {}", report.package_blake3);
    for warning in report.warnings {
        eprintln!("warning: {warning}");
    }
    Ok(())
}

fn print_help() {
    println!(r#"Vetrace Build

Export a project without invoking Cargo.

USAGE:
    vetrace-build [OPTIONS] [PROJECT]

OPTIONS:
    -p, --project <PATH>          Project directory or manifest
        --preset <NAME>          Export preset [default: configured default]
        --player-template <PATH> Prebuilt vetrace-player binary
    -h, --help                    Print help

The player template may also be supplied through VETRACE_PLAYER_TEMPLATE.
"#);
}
