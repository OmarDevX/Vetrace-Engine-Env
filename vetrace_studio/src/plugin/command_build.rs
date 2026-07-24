use super::*;

impl StudioPlugin {
    pub(super) fn apply_build_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::SaveExportPreset(preset) => {
                match self.builds.save_preset(&self.project, preset) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::InstallPlayerTemplate(path) => {
                match self.builds.install_template_binary(&path) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::InstallPlayerTemplateArchive(path) => {
                match self.builds.install_template_archive(&path) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::DownloadPlayerTemplate(url) => {
                match self.builds.download_template(&url) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::DownloadCompatiblePlayerTemplate { catalog_url, target } => {
                match self.builds.download_compatible_template(&catalog_url, target) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::RemovePlayerTemplate(id) => {
                match self.builds.remove_template(&id) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::BuildProject { preset, player_template } => {
                if self.dirty && !self.save_scene(engine) {
                    return;
                }
                if self.scripts.has_dirty_documents() && !self.save_all_scripts() {
                    return;
                }
                match self.assets.refresh() {
                    Ok(status) => self.log(status),
                    Err(error) => {
                        self.log(format!("Asset preflight failed: {error}"));
                        return;
                    }
                }
                match self.builds.start(
                    &self.project,
                    preset,
                    (!player_template.as_os_str().is_empty()).then_some(player_template),
                ) {
                    Ok(status) => { self.status = status.clone(); self.log(status); }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::OpenBuildFolder(path) => {
                if let Err(error) = StudioBuilds::open_output(&path) {
                    self.log(error);
                }
            }
            _ => unreachable!("non-build command routed to build handler"),
        }
    }
}
