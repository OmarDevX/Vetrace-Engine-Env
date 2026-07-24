use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use vetrace_project::{ProjectPath, VetraceProject};

pub const LUA_SCRIPT_COMPONENT_ID: &str = "vetrace.scripting.lua_script";
pub const LUA_SCRIPT_FIELD: &str = "script";
const LUA_SCRIPT_ROOT: &str = "assets/scripts";

pub fn is_lua_script_field(component: &str, field_path: &str) -> bool {
    component == LUA_SCRIPT_COMPONENT_ID && field_path == LUA_SCRIPT_FIELD
}

pub fn suggested_script_path(entity_name: &str) -> String {
    let stem = portable_script_stem(entity_name);
    format!("{LUA_SCRIPT_ROOT}/{stem}.lua")
}

pub fn normalize_new_script_path(requested: &str) -> Result<ProjectPath, String> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err("script path cannot be empty".to_string());
    }

    if Path::new(requested).is_absolute()
        || requested.starts_with('/')
        || requested.as_bytes().get(1) == Some(&b':')
    {
        return Err("new script path must be project-relative".to_string());
    }

    let mut normalized = requested.replace('\\', "/");
    if !normalized.starts_with("assets/") {
        normalized = format!("{LUA_SCRIPT_ROOT}/{}", normalized.trim_start_matches('/'));
    }
    if Path::new(&normalized).extension().is_none() {
        normalized.push_str(".lua");
    }

    let path = ProjectPath::new(&normalized).map_err(|error| error.to_string())?;
    validate_script_project_path(&path)?;
    validate_portable_components(&path)?;
    Ok(path)
}

pub fn resolve_existing_script(
    project: &VetraceProject,
    source: &Path,
) -> Result<(ProjectPath, PathBuf), String> {
    let project_path = project
        .paths()
        .to_project_path(source)
        .map_err(|error| error.to_string())?;
    validate_script_project_path(&project_path)?;
    let resolved = project
        .paths()
        .resolve_existing(&project_path)
        .map_err(|error| error.to_string())?;
    if !resolved.is_file() {
        return Err(format!("script is not a file: {}", resolved.display()));
    }
    Ok((project_path, resolved))
}

pub fn create_lua_script(
    project: &VetraceProject,
    requested: &str,
    entity_name: &str,
) -> Result<(ProjectPath, PathBuf), String> {
    let project_path = normalize_new_script_path(requested)?;
    let output = project
        .paths()
        .resolve_for_write(&project_path)
        .map_err(|error| error.to_string())?;
    if output.exists() {
        return Err(format!("script already exists: {}", output.display()));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create script folder '{}': {error}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|error| format!("failed to create script '{}': {error}", output.display()))?;
    file.write_all(lua_entity_script_template(entity_name).as_bytes())
        .map_err(|error| format!("failed to write script '{}': {error}", output.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to flush script '{}': {error}", output.display()))?;
    Ok((project_path, output))
}

pub fn lua_entity_script_template(entity_name: &str) -> String {
    let label = entity_name.replace('\r', " " ).replace('\n', " " );
    format!(
        "-- {label}\nreturn {{\n    properties = {{\n        -- speed = {{ type = \"number\", default = 5.0 }},\n    }},\n\n    ready = function(self)\n        -- Called once when the entity starts.\n    end,\n\n    update = function(self, dt)\n        -- Called every frame.\n    end,\n\n    fixed_update = function(self, dt)\n        -- Called on the fixed physics step.\n    end,\n\n    destroy = function(self)\n        -- Called before the script instance is removed.\n    end,\n}}\n"
    )
}

fn validate_script_project_path(path: &ProjectPath) -> Result<(), String> {
    if !path.starts_with(LUA_SCRIPT_ROOT) {
        return Err(format!("Lua scripts must be under {LUA_SCRIPT_ROOT}/"));
    }
    if path.extension().map(|extension| extension.eq_ignore_ascii_case("lua")) != Some(true) {
        return Err("Lua script path must end in .lua".to_string());
    }
    Ok(())
}

fn validate_portable_components(path: &ProjectPath) -> Result<(), String> {
    const FORBIDDEN: &[char] = &['<', '>', ':', '"', '|', '?', '*'];
    for component in path.as_str().split('/') {
        if component.chars().any(|character| FORBIDDEN.contains(&character)) {
            return Err(format!("script path contains a non-portable character: {component}"));
        }
        if component.ends_with('.') || component.ends_with(' ') {
            return Err(format!("script path component is not portable: {component}"));
        }
    }
    Ok(())
}

fn portable_script_stem(value: &str) -> String {
    let mut result = String::new();
    let mut previous_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if character.is_ascii_uppercase() && !result.is_empty() && !previous_separator {
                result.push('_');
            }
            result.push(character.to_ascii_lowercase());
            previous_separator = false;
        } else if !result.is_empty() && !previous_separator {
            result.push('_');
            previous_separator = true;
        }
    }
    let result = result.trim_matches('_');
    if result.is_empty() { "entity_script".to_string() } else { result.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_short_script_names() {
        assert_eq!(
            normalize_new_script_path("player_controller").unwrap().as_str(),
            "assets/scripts/player_controller.lua"
        );
    }

    #[test]
    fn rejects_scripts_outside_script_root() {
        let error = normalize_new_script_path("assets/textures/not_a_script.lua").unwrap_err();
        assert!(error.contains("assets/scripts"));
    }

    #[test]
    fn suggests_portable_entity_script_names() {
        assert_eq!(suggested_script_path("Player Controller"), "assets/scripts/player_controller.lua");
        assert_eq!(suggested_script_path("HTTPServer"), "assets/scripts/h_t_t_p_server.lua");
    }

    #[test]
    fn creates_and_resolves_project_script_assets() {
        let root = std::env::temp_dir().join(format!(
            "vetrace-script-asset-test-{}",
            uuid::Uuid::new_v4(),
        ));
        let project = VetraceProject::create_new(&root, "Script Test", "0.1.0").unwrap();
        let (project_path, output) =
            create_lua_script(&project, "controllers/player", "Player").unwrap();
        assert_eq!(project_path.as_str(), "assets/scripts/controllers/player.lua");
        assert!(output.is_file());
        assert!(std::fs::read_to_string(&output).unwrap().contains("update = function"));
        let (resolved_path, resolved) = resolve_existing_script(&project, &output).unwrap();
        assert_eq!(resolved_path, project_path);
        assert_eq!(resolved, std::fs::canonicalize(&output).unwrap());
        let _ = std::fs::remove_dir_all(root);
    }
}
