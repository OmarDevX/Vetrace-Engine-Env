use super::*;
use std::fs;

pub fn validate_project_files(
    manifest: &ProjectManifest,
    paths: &ProjectPaths,
) -> ValidationReport {
    let mut report = validate_manifest(manifest);

    if !paths.assets().is_dir() {
        report.push(ValidationIssue::error(
            "assets_directory_missing",
            None,
            format!(
                "assets directory '{}' does not exist",
                paths.assets().display()
            ),
        ));
    }

    check_file(
        &mut report,
        paths,
        &manifest.runtime.main_scene,
        "main_scene_missing",
        "runtime.main_scene",
    );
    for (index, script) in manifest.runtime.autoload_scripts.iter().enumerate() {
        check_file(
            &mut report,
            paths,
            script,
            "autoload_missing",
            &format!("runtime.autoload_scripts[{index}]"),
        );
    }
    if let Some(icon) = &manifest.application.icon {
        check_file(
            &mut report,
            paths,
            icon,
            "application_icon_missing",
            "application.icon",
        );
    }

    report
}

fn check_file(
    report: &mut ValidationReport,
    paths: &ProjectPaths,
    project_path: &crate::ProjectPath,
    code: &'static str,
    field: &str,
) {
    let path = paths.resolve(project_path);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() => {
            if let Err(error) = paths.resolve_existing(project_path) {
                report.push(ValidationIssue::error(
                    "path_escape_symlink",
                    Some(field.to_owned()),
                    error.to_string(),
                ));
            }
        }
        Ok(_) => report.push(ValidationIssue::error(
            code,
            Some(field.to_owned()),
            format!("'{}' exists but is not a file", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            report.push(ValidationIssue::error(
                code,
                Some(field.to_owned()),
                format!("required file '{}' does not exist", path.display()),
            ));
        }
        Err(error) => report.push(ValidationIssue::error(
            code,
            Some(field.to_owned()),
            format!("could not inspect '{}': {error}", path.display()),
        )),
    }
}
