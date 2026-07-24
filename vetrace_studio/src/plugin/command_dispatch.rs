use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandDomain {
    Entity,
    Scene,
    Debug,
    Script,
    Asset,
    Build,
    Project,
}

impl StudioCommand {
    fn domain(&self) -> CommandDomain {
        match self {
            StudioCommand::Select(..)
            | StudioCommand::SetField { .. }
            | StudioCommand::AddComponent { .. }
            | StudioCommand::RemoveComponent { .. }
            | StudioCommand::Rename { .. }
            | StudioCommand::SpawnEmpty
            | StudioCommand::SpawnPrimitive(..)
            | StudioCommand::DeleteSelected => CommandDomain::Entity,
            #[cfg(feature = "render_2d")]
            StudioCommand::SpawnSprite2D
            | StudioCommand::SpawnSprite2DFromAsset { .. }
            | StudioCommand::SetViewportMode(..) => CommandDomain::Entity,
            StudioCommand::Undo
            | StudioCommand::Redo
            | StudioCommand::SaveScene
            | StudioCommand::NewScene(..)
            | StudioCommand::OpenScene(..)
            | StudioCommand::SaveSceneAs(..)
            | StudioCommand::RecoverSession
            | StudioCommand::DiscardRecovery
            | StudioCommand::SetCurrentSceneAsMain
            | StudioCommand::ReloadScene
            | StudioCommand::ReloadSceneDiscard
            | StudioCommand::SaveAndReload => CommandDomain::Scene,
            StudioCommand::PlayProject
            | StudioCommand::DebugProject
            | StudioCommand::DebugCommand(..)
            | StudioCommand::ToggleBreakpoint { .. }
            | StudioCommand::SetDebuggerWatches(..)
            | StudioCommand::SetBreakOnError(..)
            | StudioCommand::StopProject => CommandDomain::Debug,
            StudioCommand::OpenScript(..)
            | StudioCommand::OpenScriptAt { .. }
            | StudioCommand::SaveScript(..)
            | StudioCommand::SaveAndCloseScript(..)
            | StudioCommand::SaveAllScripts
            | StudioCommand::RenameScript { .. }
            | StudioCommand::DeleteScript { .. }
            | StudioCommand::ResolveScriptExternal { .. }
            | StudioCommand::AssignLuaScript { .. }
            | StudioCommand::CreateLuaScript { .. } => CommandDomain::Script,
            StudioCommand::RefreshAssets
            | StudioCommand::ImportAssetFiles(..)
            | StudioCommand::ReimportAsset(..)
            | StudioCommand::ReimportAllAssets
            | StudioCommand::ClearAssetCache
            | StudioCommand::PruneAssetCache => CommandDomain::Asset,
            StudioCommand::SaveExportPreset(..)
            | StudioCommand::InstallPlayerTemplate(..)
            | StudioCommand::InstallPlayerTemplateArchive(..)
            | StudioCommand::DownloadPlayerTemplate(..)
            | StudioCommand::DownloadCompatiblePlayerTemplate { .. }
            | StudioCommand::RemovePlayerTemplate(..)
            | StudioCommand::BuildProject { .. }
            | StudioCommand::OpenBuildFolder(..) => CommandDomain::Build,
            StudioCommand::SaveProjectSettings(..)
            | StudioCommand::OpenProjectManager
            | StudioCommand::OpenProjectManagerDiscard
            | StudioCommand::SaveAndOpenProjectManager
            | StudioCommand::Quit
            | StudioCommand::QuitDiscard
            | StudioCommand::SaveAndQuit => CommandDomain::Project,
        }
    }
}

impl StudioPlugin {
    pub(super) fn apply_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command.domain() {
            CommandDomain::Entity => self.apply_entity_command(engine, command),
            CommandDomain::Scene => self.apply_scene_command(engine, command),
            CommandDomain::Debug => self.apply_debug_command(engine, command),
            CommandDomain::Script => self.apply_script_command(engine, command),
            CommandDomain::Asset => self.apply_asset_command(engine, command),
            CommandDomain::Build => self.apply_build_command(engine, command),
            CommandDomain::Project => self.apply_project_command(engine, command),
        }
    }
}
