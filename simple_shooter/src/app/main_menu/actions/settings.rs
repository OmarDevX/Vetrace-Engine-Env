use super::*;

pub(super) fn apply(engine: &mut Engine, runtime: &mut ShooterRuntime, action: MainMenuAction) {
    match action {
        MainMenuAction::ToggleVignette => {
            update_game_settings(engine, |settings| settings.vignette = !settings.vignette);
        }
        MainMenuAction::ToggleVolumetricFog => {
            update_game_settings(engine, |settings| settings.volumetric_fog = !settings.volumetric_fog);
        }
        MainMenuAction::CycleGraphics => {
            update_game_settings(engine, |settings| settings.graphics_profile = match settings.graphics_profile {
                ShooterGraphicsProfile::LowSpec => ShooterGraphicsProfile::Balanced,
                ShooterGraphicsProfile::Balanced => ShooterGraphicsProfile::HighQuality,
                ShooterGraphicsProfile::HighQuality => ShooterGraphicsProfile::LowSpec,
            });
        }
        MainMenuAction::ToggleVsync => update_game_settings(engine, |settings| settings.vsync = !settings.vsync),
        MainMenuAction::SensitivityDown => update_game_settings(engine, |settings| settings.mouse_sensitivity -= 0.0004),
        MainMenuAction::SensitivityUp => update_game_settings(engine, |settings| settings.mouse_sensitivity += 0.0004),
        MainMenuAction::VolumeDown => update_game_settings(engine, |settings| settings.master_volume -= 0.1),
        MainMenuAction::VolumeUp => update_game_settings(engine, |settings| settings.master_volume += 0.1),
        MainMenuAction::ResetSettings => {
            if let Some(settings) = engine.get_resource_mut::<ShooterGameSettings>() { *settings = ShooterGameSettings::default(); }
            apply_and_save_game_settings(engine);
        }
        _ => unreachable!("non-settings main-menu action routed to settings handler"),
    }
}
