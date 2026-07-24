use super::*;

mod session;
mod navigation;
mod map_setup;
mod mods;
mod settings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainMenuActionDomain {
    Session,
    Navigation,
    MapSetup,
    Mods,
    Settings,
}

impl MainMenuAction {
    fn domain(self) -> MainMenuActionDomain {
        match self {
            MainMenuAction::Play
            | MainMenuAction::Servers
            | MainMenuAction::RefreshServers
            | MainMenuAction::HostServer
            | MainMenuAction::PreviousServer
            | MainMenuAction::NextServer
            | MainMenuAction::JoinServer => MainMenuActionDomain::Session,
            MainMenuAction::RandomizePlayer
            | MainMenuAction::Maps
            | MainMenuAction::Shop
            | MainMenuAction::Mods
            | MainMenuAction::Settings
            | MainMenuAction::ClosePage => MainMenuActionDomain::Navigation,
            MainMenuAction::PreviousMap
            | MainMenuAction::NextMap
            | MainMenuAction::PreviousBotDifficulty
            | MainMenuAction::NextBotDifficulty
            | MainMenuAction::BotCountDown
            | MainMenuAction::BotCountUp
            | MainMenuAction::MaxPlayersDown
            | MainMenuAction::MaxPlayersUp => MainMenuActionDomain::MapSetup,
            MainMenuAction::ToggleMod
            | MainMenuAction::PreviousMod
            | MainMenuAction::NextMod
            | MainMenuAction::ReloadMod => MainMenuActionDomain::Mods,
            MainMenuAction::ToggleVignette
            | MainMenuAction::ToggleVolumetricFog
            | MainMenuAction::CycleGraphics
            | MainMenuAction::ToggleVsync
            | MainMenuAction::SensitivityDown
            | MainMenuAction::SensitivityUp
            | MainMenuAction::VolumeDown
            | MainMenuAction::VolumeUp
            | MainMenuAction::ResetSettings => MainMenuActionDomain::Settings,
        }
    }
}

pub(crate) fn apply_main_menu_action(
    engine: &mut Engine,
    runtime: &mut ShooterRuntime,
    action: MainMenuAction,
) {
    if runtime.background_hosting
        && matches!(action, MainMenuAction::Play | MainMenuAction::HostServer | MainMenuAction::JoinServer)
    {
        if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
            menu.status = "Your server is still running for the remaining players.".to_string();
        }
        return;
    }

    match action.domain() {
        MainMenuActionDomain::Session => session::apply(engine, runtime, action),
        MainMenuActionDomain::Navigation => navigation::apply(engine, runtime, action),
        MainMenuActionDomain::MapSetup => map_setup::apply(engine, runtime, action),
        MainMenuActionDomain::Mods => mods::apply(engine, runtime, action),
        MainMenuActionDomain::Settings => settings::apply(engine, runtime, action),
    }
}
