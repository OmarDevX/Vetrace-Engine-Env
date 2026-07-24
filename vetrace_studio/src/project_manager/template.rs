use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use vetrace_primitives::PrimitiveKind;
use vetrace_project::{InputAction, ProjectManifest, VetraceProject};
use vetrace_scene::{
    component_type, save_scene_file, SceneComponent, SceneDocument, SceneMaterial, SceneNode,
    ScenePrimitive, SceneTransform,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectTemplate {
    Empty,
    #[default]
    Starter3d,
    LuaStarter,
}

impl ProjectTemplate {
    pub const ALL: [Self; 3] = [Self::Empty, Self::Starter3d, Self::LuaStarter];

    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "Empty project",
            Self::Starter3d => "3D starter",
            Self::LuaStarter => "Lua starter",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Empty => "A valid project with an empty main scene.",
            Self::Starter3d => "A ground plane and a visible starter cube.",
            Self::LuaStarter => "A Lua-controlled cube, input actions, and a ground plane.",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CreateProjectRequest {
    pub name: String,
    pub parent_directory: PathBuf,
    pub folder_name: String,
    pub template: ProjectTemplate,
}

impl CreateProjectRequest {
    pub fn target_directory(&self) -> PathBuf {
        self.parent_directory.join(self.folder_name.trim())
    }

    pub fn validate(&self) -> Result<PathBuf, String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Project name cannot be empty".to_string());
        }
        if name.len() > 128 {
            return Err("Project name cannot exceed 128 characters".to_string());
        }
        if self.parent_directory.as_os_str().is_empty() {
            return Err("Choose a parent directory".to_string());
        }
        let folder = self.folder_name.trim();
        if folder.is_empty() {
            return Err("Project folder cannot be empty".to_string());
        }
        if folder == "." || folder == ".." || folder.contains('/') || folder.contains('\\') {
            return Err("Project folder must be a single directory name".to_string());
        }
        let target = self.target_directory();
        if target.join(vetrace_project::PROJECT_MANIFEST_FILE).exists() {
            return Err(format!("A Vetrace project already exists at '{}'", target.display()));
        }
        if target.exists() {
            let mut entries = fs::read_dir(&target)
                .map_err(|error| format!("failed to inspect '{}': {error}", target.display()))?;
            if entries.next().is_some() {
                return Err(format!("Target directory '{}' is not empty", target.display()));
            }
        }
        Ok(target)
    }
}

pub fn create_project(request: &CreateProjectRequest) -> Result<VetraceProject, String> {
    let root = request.validate()?;
    fs::create_dir_all(&root)
        .map_err(|error| format!("failed to create project directory '{}': {error}", root.display()))?;

    let mut manifest = ProjectManifest::new(request.name.trim(), env!("CARGO_PKG_VERSION"));
    manifest.application.cursor_grab = false;
    manifest.application.cursor_visible = true;
    manifest.features.networking = false;
    manifest.features.audio = false;
    manifest.features.animation = false;

    if request.template == ProjectTemplate::LuaStarter {
        manifest.input.insert(
            "move_left",
            InputAction {
                keys: vec!["A".to_string(), "ArrowLeft".to_string()],
                ..InputAction::default()
            },
        );
        manifest.input.insert(
            "move_right",
            InputAction {
                keys: vec!["D".to_string(), "ArrowRight".to_string()],
                ..InputAction::default()
            },
        );
    }

    let project = match VetraceProject::create(&root, manifest) {
        Ok(project) => project,
        Err(error) => {
            let _ = remove_directory_if_empty(&root);
            return Err(error.to_string());
        }
    };

    let result = (|| {
        let scene = template_scene(request.template, request.name.trim());
        save_scene_file(project.main_scene_path(), &scene)
            .map_err(|error| format!("failed to create main scene: {error}"))?;
        if request.template == ProjectTemplate::LuaStarter {
            fs::write(project.paths().scripts().join("player.lua"), LUA_STARTER_SCRIPT)
                .map_err(|error| format!("failed to create Lua starter script: {error}"))?;
        }
        fs::write(
            project.root().join(".gitignore"),
            ".vetrace/cache/\n.vetrace/imported/\nbuilds/\n",
        )
        .map_err(|error| format!("failed to create project .gitignore: {error}"))?;
        project.validate_files().into_result().map_err(|error| error.to_string())?;
        Ok::<(), String>(())
    })();

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }
    Ok(project)
}

pub struct TemporaryManagerProject {
    pub project: VetraceProject,
    root: PathBuf,
}

impl Drop for TemporaryManagerProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn create_temporary_manager_project() -> Result<TemporaryManagerProject, String> {
    let root = std::env::temp_dir().join(format!(
        "vetrace-studio-project-manager-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let mut manifest = ProjectManifest::new("Vetrace Studio", env!("CARGO_PKG_VERSION"));
    manifest.application.title = "Vetrace Studio — Project Manager".to_string();
    manifest.application.width = 1120;
    manifest.application.height = 720;
    manifest.application.cursor_grab = false;
    manifest.application.cursor_visible = true;
    manifest.features.physics = false;
    manifest.features.audio = false;
    manifest.features.animation = false;
    manifest.features.networking = false;
    manifest.features.ui = false;
    manifest.features.scripting = false;
    let project = match VetraceProject::create(&root, manifest) {
        Ok(project) => project,
        Err(error) => {
            let _ = fs::remove_dir_all(&root);
            return Err(error.to_string());
        }
    };
    if let Err(error) = save_scene_file(project.main_scene_path(), &SceneDocument::new("Project Manager")) {
        let _ = fs::remove_dir_all(&root);
        return Err(format!("failed to create project-manager scene: {error}"));
    }
    Ok(TemporaryManagerProject { project, root })
}

pub fn slugify_project_name(name: &str) -> String {
    let mut output = String::new();
    let mut pending_separator = false;
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            output.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else if !output.is_empty() {
            pending_separator = true;
        }
    }
    if output.is_empty() { "new-vetrace-project".to_string() } else { output }
}

fn template_scene(template: ProjectTemplate, name: &str) -> SceneDocument {
    let mut scene = SceneDocument::new(format!("{name} Main"));
    match template {
        ProjectTemplate::Empty => {}
        ProjectTemplate::Starter3d => {
            scene.roots.push(primitive_node(
                "Starter Cube",
                PrimitiveKind::Cube,
                [0.0, 0.0, 0.0],
                [1.25, 1.25, 1.25],
                [0.92, 0.35, 0.12],
                None,
            ));
            scene.roots.push(ground_node());
        }
        ProjectTemplate::LuaStarter => {
            scene.roots.push(primitive_node(
                "Lua Player",
                PrimitiveKind::Cube,
                [0.0, 0.0, 0.0],
                [1.25, 1.25, 1.25],
                [0.92, 0.35, 0.12],
                Some(SceneComponent::raw(
                    "vetrace.scripting.lua_script",
                    json!({
                        "script": "assets/scripts/player.lua",
                        "enabled": true,
                        "properties": { "speed": 4.0 }
                    }),
                )),
            ));
            scene.roots.push(ground_node());
        }
    }
    scene
}

fn ground_node() -> SceneNode {
    primitive_node(
        "Ground",
        PrimitiveKind::Cube,
        [0.0, -1.15, 0.0],
        [10.0, 0.4, 6.0],
        [0.16, 0.22, 0.30],
        None,
    )
}

fn primitive_node(
    name: &str,
    kind: PrimitiveKind,
    translation: [f32; 3],
    size: [f32; 3],
    color: [f32; 3],
    extra: Option<SceneComponent>,
) -> SceneNode {
    let mut components = vec![
        SceneComponent::new(
            component_type::PRIMITIVE,
            ScenePrimitive { kind, size, visible: true },
        ),
        SceneComponent::new(
            component_type::MATERIAL,
            SceneMaterial {
                base_color: color,
                roughness: 0.72,
                metallic: 0.02,
                ..SceneMaterial::default()
            },
        ),
    ];
    if let Some(extra) = extra {
        components.push(extra);
    }
    SceneNode {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        transform: SceneTransform { translation, ..SceneTransform::default() },
        components,
        children: Vec::new(),
    }
}

fn remove_directory_if_empty(path: &Path) -> std::io::Result<()> {
    if path.is_dir() && fs::read_dir(path)?.next().is_none() {
        fs::remove_dir(path)?;
    }
    Ok(())
}

const LUA_STARTER_SCRIPT: &str = r#"return {
    properties = {
        speed = { type = "number", default = 4.0 },
    },

    ready = function(self)
        Debug.log("Lua starter ready — move with A/D or Left/Right")
    end,

    update = function(self, dt)
        local direction = 0.0
        if Input.action_down("move_left") then
            direction = direction - 1.0
        end
        if Input.action_down("move_right") then
            direction = direction + 1.0
        end
        self.components.Transform.translation.x =
            self.components.Transform.translation.x + direction * self.speed * dt
    end,
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_portable_and_stable() {
        assert_eq!(slugify_project_name("My Great Game!"), "my-great-game");
        assert_eq!(slugify_project_name("  "), "new-vetrace-project");
    }

    #[test]
    fn lua_template_creates_a_runnable_project() {
        let parent = std::env::temp_dir().join(format!("vetrace-template-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&parent).unwrap();
        let request = CreateProjectRequest {
            name: "Template Test".to_string(),
            parent_directory: parent.clone(),
            folder_name: "template-test".to_string(),
            template: ProjectTemplate::LuaStarter,
        };
        let project = create_project(&request).unwrap();
        assert!(project.main_scene_path().is_file());
        assert!(project.paths().scripts().join("player.lua").is_file());
        assert!(project.validate_files().is_valid());
        let _ = fs::remove_dir_all(parent);
    }
}
