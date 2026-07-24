use super::*;
use std::collections::BTreeSet;

pub fn validate_manifest(manifest: &ProjectManifest) -> ValidationReport {
    let mut report = ValidationReport::default();
    validate_format_and_identity(manifest, &mut report);
    validate_application(manifest, &mut report);
    validate_runtime_and_features(manifest, &mut report);
    validate_rendering_and_physics(manifest, &mut report);
    validate_scripting(manifest, &mut report);
    validate_input(manifest, &mut report);
    report
}

fn validate_format_and_identity(manifest: &ProjectManifest, report: &mut ValidationReport) {
    if manifest.format_version != CURRENT_PROJECT_FORMAT_VERSION {
        let relation = if manifest.format_version > CURRENT_PROJECT_FORMAT_VERSION {
            "newer than this engine supports"
        } else {
            "older than the current format and requires migration"
        };
        report.push(ValidationIssue::error(
            "format_version",
            Some("format_version".to_owned()),
            format!(
                "format {} is {relation}; supported format is {}",
                manifest.format_version, CURRENT_PROJECT_FORMAT_VERSION
            ),
        ));
    }

    if manifest.project.id.is_nil() {
        report.push(ValidationIssue::error(
            "project_id_nil",
            Some("project.id".to_owned()),
            "project ID cannot be the nil UUID",
        ));
    }
    validate_required_text(report, "project.name", &manifest.project.name, 128);
    validate_required_text(report, "project.version", &manifest.project.version, 64);
    validate_required_text(
        report,
        "project.engine_version",
        &manifest.project.engine_version,
        64,
    );
    if !looks_like_semver(&manifest.project.version) {
        report.push(ValidationIssue::warning(
            "project_version_non_semver",
            Some("project.version".to_owned()),
            "version is not in the recommended major.minor.patch form",
        ));
    }
}

fn validate_application(manifest: &ProjectManifest, report: &mut ValidationReport) {
    validate_required_text(
        report,
        "application.title",
        &manifest.application.title,
        256,
    );
    if let Some(icon) = &manifest.application.icon {
        if !icon.starts_with("assets") {
            report.push(ValidationIssue::error(
                "application_icon_outside_assets",
                Some("application.icon".to_owned()),
                "application icon must be stored under assets/",
            ));
        }
        if !matches!(icon.extension(), Some("png" | "jpg" | "jpeg" | "ico")) {
            report.push(ValidationIssue::warning(
                "application_icon_extension",
                Some("application.icon".to_owned()),
                "application icon should use .png, .jpg, .jpeg, or .ico",
            ));
        }
    }
    validate_window_dimension(
        report,
        "application.width",
        "window_width_range",
        manifest.application.width,
        "width",
    );
    validate_window_dimension(
        report,
        "application.height",
        "window_height_range",
        manifest.application.height,
        "height",
    );
}

fn validate_window_dimension(
    report: &mut ValidationReport,
    field: &str,
    code: &'static str,
    value: u32,
    label: &str,
) {
    if !(64..=16384).contains(&value) {
        report.push(ValidationIssue::error(
            code,
            Some(field.to_owned()),
            format!("window {label} must be between 64 and 16384 pixels"),
        ));
    }
}

fn validate_runtime_and_features(manifest: &ProjectManifest, report: &mut ValidationReport) {
    if !manifest.runtime.main_scene.starts_with("assets") {
        report.push(ValidationIssue::error(
            "main_scene_outside_assets",
            Some("runtime.main_scene".to_owned()),
            "main scene must be stored under assets/",
        ));
    }
    if manifest.runtime.main_scene.extension() != Some("vscene") {
        report.push(ValidationIssue::warning(
            "main_scene_extension",
            Some("runtime.main_scene".to_owned()),
            "main scene should use the .vscene extension",
        ));
    }

    let mut scripts = BTreeSet::new();
    for (index, script) in manifest.runtime.autoload_scripts.iter().enumerate() {
        let field = format!("runtime.autoload_scripts[{index}]");
        if !script.starts_with("assets") {
            report.push(ValidationIssue::error(
                "autoload_outside_assets",
                Some(field.clone()),
                "autoload scripts must be stored under assets/",
            ));
        }
        if script.extension() != Some("lua") {
            report.push(ValidationIssue::error(
                "autoload_extension",
                Some(field.clone()),
                "autoload script must use the .lua extension",
            ));
        }
        if !scripts.insert(script.as_str()) {
            report.push(ValidationIssue::warning(
                "autoload_duplicate",
                Some(field),
                format!("autoload script '{}' is listed more than once", script),
            ));
        }
    }
    if !manifest.features.scripting && !manifest.runtime.autoload_scripts.is_empty() {
        report.push(ValidationIssue::error(
            "autoload_without_scripting",
            Some("features.scripting".to_owned()),
            "scripting cannot be disabled while autoload scripts are configured",
        ));
    }
    if !manifest.features.rendering && manifest.features.ui {
        report.push(ValidationIssue::warning(
            "ui_without_rendering",
            Some("features.ui".to_owned()),
            "UI is enabled while rendering is disabled; this is only useful for unusual headless configurations",
        ));
    }
}

fn validate_rendering_and_physics(manifest: &ProjectManifest, report: &mut ValidationReport) {
    if !matches!(manifest.rendering.msaa_samples, 1 | 2 | 4 | 8) {
        report.push(ValidationIssue::error(
            "msaa_samples",
            Some("rendering.msaa_samples".to_owned()),
            "MSAA samples must be one of 1, 2, 4, or 8",
        ));
    }
    if !manifest.rendering.render_scale.is_finite()
        || !(0.1..=4.0).contains(&manifest.rendering.render_scale)
    {
        report.push(ValidationIssue::error(
            "render_scale",
            Some("rendering.render_scale".to_owned()),
            "render scale must be finite and between 0.1 and 4.0",
        ));
    }

    if manifest
        .physics
        .gravity
        .iter()
        .any(|value| !value.is_finite())
    {
        report.push(ValidationIssue::error(
            "gravity_finite",
            Some("physics.gravity".to_owned()),
            "gravity components must be finite",
        ));
    }
    if !manifest.physics.fixed_timestep.is_finite()
        || !(0.000_1..=1.0).contains(&manifest.physics.fixed_timestep)
    {
        report.push(ValidationIssue::error(
            "fixed_timestep",
            Some("physics.fixed_timestep".to_owned()),
            "fixed timestep must be finite and between 0.0001 and 1.0 seconds",
        ));
    }
    if !(1..=64).contains(&manifest.physics.max_substeps) {
        report.push(ValidationIssue::error(
            "max_substeps",
            Some("physics.max_substeps".to_owned()),
            "max substeps must be between 1 and 64",
        ));
    }
}

fn validate_scripting(manifest: &ProjectManifest, report: &mut ValidationReport) {
    if manifest.scripting.max_errors_per_frame == 0 {
        report.push(ValidationIssue::error(
            "script_error_budget",
            Some("scripting.max_errors_per_frame".to_owned()),
            "max errors per frame must be at least 1",
        ));
    }
}

fn validate_input(manifest: &ProjectManifest, report: &mut ValidationReport) {
    for (name, action) in &manifest.input.actions {
        let base = format!("input.actions.{name}");
        if !valid_action_name(name) {
            report.push(ValidationIssue::error(
                "input_action_name",
                Some(base.clone()),
                "action names must start with a letter and contain only lowercase letters, digits, '_' or '-'",
            ));
        }
        if action.is_unbound() {
            report.push(ValidationIssue::warning(
                "input_action_unbound",
                Some(base.clone()),
                "input action has no bindings",
            ));
        }
        if !action.dead_zone.is_finite() || !(0.0..=1.0).contains(&action.dead_zone) {
            report.push(ValidationIssue::error(
                "input_dead_zone",
                Some(format!("{base}.dead_zone")),
                "dead zone must be finite and between 0.0 and 1.0",
            ));
        }
        warn_duplicates(report, &format!("{base}.keys"), &action.keys);
        warn_duplicates(
            report,
            &format!("{base}.mouse_buttons"),
            &action.mouse_buttons,
        );
        warn_duplicates(
            report,
            &format!("{base}.gamepad_buttons"),
            &action.gamepad_buttons,
        );
        for (index, axis) in action.axes.iter().enumerate() {
            if axis.axis.trim().is_empty() {
                report.push(ValidationIssue::error(
                    "input_axis_empty",
                    Some(format!("{base}.axes[{index}].axis")),
                    "axis name cannot be empty",
                ));
            }
            if !axis.scale.is_finite() || axis.scale == 0.0 {
                report.push(ValidationIssue::error(
                    "input_axis_scale",
                    Some(format!("{base}.axes[{index}].scale")),
                    "axis scale must be finite and non-zero",
                ));
            }
        }
    }
}
