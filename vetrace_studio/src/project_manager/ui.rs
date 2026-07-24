use std::path::{Path, PathBuf};

use rfd::FileDialog;
use vetrace_render::{egui, EguiTool, EguiToolContext};

use super::{
    default_projects_directory, slugify_project_name, CreateProjectRequest, ProjectManagerBridge,
    ProjectManagerCommand, ProjectTemplate,
};

const ACTION_PANEL_DEFAULT_WIDTH: f32 = 420.0;
const ACTION_PANEL_MIN_WIDTH: f32 = 320.0;
const ACTION_PANEL_MAX_WIDTH: f32 = 540.0;

pub struct ProjectManagerEguiTool {
    bridge: ProjectManagerBridge,
    open_path: String,
    project_name: String,
    parent_directory: String,
    folder_name: String,
    folder_manually_edited: bool,
    template: ProjectTemplate,
}

impl ProjectManagerEguiTool {
    pub fn new(bridge: ProjectManagerBridge) -> Self {
        let parent = default_projects_directory();
        Self {
            bridge,
            open_path: String::new(),
            project_name: "My Vetrace Game".to_string(),
            parent_directory: parent.display().to_string(),
            folder_name: "my-vetrace-game".to_string(),
            folder_manually_edited: false,
            template: ProjectTemplate::Starter3d,
        }
    }

    fn command(&self, command: ProjectManagerCommand) {
        self.bridge.push(command);
    }

    fn browse_project_folder(&mut self) {
        let mut dialog = FileDialog::new().set_title("Open Vetrace Project Folder");
        if let Some(directory) = dialog_start_directory(&self.open_path) {
            dialog = dialog.set_directory(directory);
        }
        if let Some(path) = dialog.pick_folder() {
            self.open_path = path.display().to_string();
        }
    }

    fn browse_project_manifest(&mut self) {
        let mut dialog = FileDialog::new()
            .set_title("Select project.vetrace.toml")
            .add_filter("Vetrace project", &["toml"])
            .set_file_name("project.vetrace.toml");
        if let Some(directory) = dialog_start_directory(&self.open_path) {
            dialog = dialog.set_directory(directory);
        }
        if let Some(path) = dialog.pick_file() {
            self.open_path = path.display().to_string();
        }
    }

    fn browse_parent_directory(&mut self) {
        let mut dialog = FileDialog::new().set_title("Choose Projects Directory");
        if let Some(directory) = dialog_start_directory(&self.parent_directory) {
            dialog = dialog.set_directory(directory);
        }
        if let Some(path) = dialog.pick_folder() {
            self.parent_directory = path.display().to_string();
        }
    }
}

impl EguiTool for ProjectManagerEguiTool {
    fn ui(&mut self, ctx: &egui::Context, _frame: &EguiToolContext) {
        let snapshot = self
            .bridge
            .snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default();
        self.draw_header(ctx);
        self.draw_actions(ctx, &snapshot);
        self.draw_status(ctx, &snapshot);
        self.draw_recent_projects(ctx, &snapshot);
    }
}

impl ProjectManagerEguiTool {
    fn draw_header(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("vetrace_project_manager_header")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.heading("Vetrace Studio");
                    ui.separator();
                    ui.label("Project Manager");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Quit").clicked() {
                            self.command(ProjectManagerCommand::Quit);
                        }
                        if ui.button("Refresh").clicked() {
                            self.command(ProjectManagerCommand::Refresh);
                        }
                    });
                });
                ui.add_space(6.0);
            });
    }

    fn draw_actions(&mut self, ctx: &egui::Context, snapshot: &super::ProjectManagerSnapshot) {
        egui::SidePanel::right("vetrace_project_manager_actions")
            .default_width(ACTION_PANEL_DEFAULT_WIDTH)
            .min_width(ACTION_PANEL_MIN_WIDTH)
            .max_width(ACTION_PANEL_MAX_WIDTH)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.draw_open_project(ui, snapshot);
                        ui.add_space(22.0);
                        ui.separator();
                        ui.add_space(12.0);
                        self.draw_create_project(ui, snapshot);
                    });
            });
    }

    fn draw_open_project(&mut self, ui: &mut egui::Ui, snapshot: &super::ProjectManagerSnapshot) {
        ui.heading("Open existing project");
        ui.label("Browse to any project folder, select project.vetrace.toml, or paste a path.");
        ui.add_sized(
            [ui.available_width(), ui.spacing().interact_size.y],
            egui::TextEdit::singleline(&mut self.open_path)
                .hint_text("/path/to/MyGame or project.vetrace.toml"),
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!snapshot.busy, egui::Button::new("Browse folder…"))
                .clicked()
            {
                self.browse_project_folder();
            }
            if ui
                .add_enabled(!snapshot.busy, egui::Button::new("Select project file…"))
                .clicked()
            {
                self.browse_project_manifest();
            }
        });
        let can_open = !self.open_path.trim().is_empty() && !snapshot.busy;
        if ui
            .add_enabled(can_open, egui::Button::new("Open / Import"))
            .clicked()
        {
            self.command(ProjectManagerCommand::Open(PathBuf::from(
                self.open_path.trim(),
            )));
        }
    }

    fn draw_create_project(&mut self, ui: &mut egui::Ui, snapshot: &super::ProjectManagerSnapshot) {
        ui.heading("Create project");
        ui.label("Project name");
        if ui.text_edit_singleline(&mut self.project_name).changed()
            && !self.folder_manually_edited
        {
            self.folder_name = slugify_project_name(&self.project_name);
        }

        ui.label("Parent directory");
        ui.add_sized(
            [ui.available_width(), ui.spacing().interact_size.y],
            egui::TextEdit::singleline(&mut self.parent_directory),
        );
        if ui
            .add_enabled(!snapshot.busy, egui::Button::new("Browse parent folder…"))
            .clicked()
        {
            self.browse_parent_directory();
        }

        ui.label("Project folder");
        if ui.text_edit_singleline(&mut self.folder_name).changed() {
            self.folder_manually_edited = true;
        }
        ui.label("Template");
        egui::ComboBox::from_id_source("vetrace_project_template")
            .selected_text(self.template.label())
            .show_ui(ui, |ui| {
                for template in ProjectTemplate::ALL {
                    ui.selectable_value(&mut self.template, template, template.label());
                }
            });
        ui.label(egui::RichText::new(self.template.description()).small().weak());

        let request = CreateProjectRequest {
            name: self.project_name.trim().to_string(),
            parent_directory: PathBuf::from(self.parent_directory.trim()),
            folder_name: self.folder_name.trim().to_string(),
            template: self.template,
        };
        ui.add_space(8.0);
        ui.label("Project path");
        ui.add(
            egui::Label::new(
                egui::RichText::new(request.target_directory().display().to_string()).monospace(),
            )
            .wrap(),
        );
        ui.add_space(8.0);
        let validation = request.validate();
        if let Err(error) = &validation {
            ui.label(egui::RichText::new(error).small().weak());
        }
        if ui
            .add_enabled(validation.is_ok() && !snapshot.busy, egui::Button::new("Create and open"))
            .clicked()
        {
            self.command(ProjectManagerCommand::Create(request));
        }
    }

    fn draw_status(&self, ctx: &egui::Context, snapshot: &super::ProjectManagerSnapshot) {
        egui::TopBottomPanel::bottom("vetrace_project_manager_status")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if snapshot.busy {
                        ui.spinner();
                    }
                    ui.label(&snapshot.status);
                });
            });
    }

    fn draw_recent_projects(&self, ctx: &egui::Context, snapshot: &super::ProjectManagerSnapshot) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Recent projects");
            ui.label("Open a recent project or remove entries that are no longer needed.");
            ui.add_space(8.0);
            if snapshot.recent.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading("No recent projects");
                    ui.label("Create a project or browse to an existing project on the right.");
                });
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for project in &snapshot.recent {
                    self.draw_recent_project(ui, project, snapshot.busy);
                    ui.add_space(6.0);
                }
            });
        });
    }

    fn draw_recent_project(
        &self,
        ui: &mut egui::Ui,
        project: &super::RecentProject,
        busy: bool,
    ) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.strong(&project.name);
                    ui.monospace(project.path.display().to_string());
                    let metadata = if project.version.is_empty() {
                        project.detail.clone()
                    } else {
                        format!(
                            "Project {} · engine {} · {}",
                            project.version, project.engine_version, project.detail
                        )
                    };
                    let label = if project.valid {
                        egui::RichText::new(metadata).small()
                    } else {
                        egui::RichText::new(metadata).small().weak()
                    };
                    ui.label(label);
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Remove").clicked() {
                        self.command(ProjectManagerCommand::RemoveRecent(project.path.clone()));
                    }
                    if ui
                        .add_enabled(project.available && project.valid && !busy, egui::Button::new("Open"))
                        .clicked()
                    {
                        self.command(ProjectManagerCommand::Open(project.path.clone()));
                    }
                });
            });
        });
    }
}

fn dialog_start_directory(input: &str) -> Option<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path = PathBuf::from(trimmed);
    if path.file_name().and_then(|name| name.to_str()) == Some("project.vetrace.toml")
        || path.is_file()
    {
        return path.parent().map(Path::to_path_buf);
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::dialog_start_directory;
    use std::path::PathBuf;

    #[test]
    fn manifest_paths_start_dialog_in_parent_directory() {
        let start = dialog_start_directory("/tmp/MyGame/project.vetrace.toml");
        assert_eq!(start, Some(PathBuf::from("/tmp/MyGame")));
    }

    #[test]
    fn project_directories_are_used_directly() {
        let start = dialog_start_directory("/tmp/MyGame");
        assert_eq!(start, Some(PathBuf::from("/tmp/MyGame")));
    }
}
