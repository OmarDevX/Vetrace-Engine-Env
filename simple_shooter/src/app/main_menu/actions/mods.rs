use super::*;

pub(super) fn apply(engine: &mut Engine, runtime: &mut ShooterRuntime, action: MainMenuAction) {
    match action {
        MainMenuAction::ToggleMod => {
            let selected = engine.get_resource::<MainMenuState>().map(|menu| menu.selected_mod).unwrap_or(0);
            let status = toggle_selected_shooter_mod(engine, selected);
            let enabled = selected_shooter_mod(engine, selected).map(|info| info.enabled).unwrap_or(false);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                menu.mod_enabled = enabled;
                menu.status = status;
            }
            show_main_menu_page(engine, MainMenuPage::Mods);
        }
        MainMenuAction::PreviousMod => {
            let count = shooter_mod_count(engine);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                if count > 0 { menu.selected_mod = (menu.selected_mod + count - 1) % count; }
            }
            show_main_menu_page(engine, MainMenuPage::Mods);
        }
        MainMenuAction::NextMod => {
            let count = shooter_mod_count(engine);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                if count > 0 { menu.selected_mod = (menu.selected_mod + 1) % count; }
            }
            show_main_menu_page(engine, MainMenuPage::Mods);
        }
        MainMenuAction::ReloadMod => {
            let selected = engine.get_resource::<MainMenuState>().map(|menu| menu.selected_mod).unwrap_or(0);
            let status = reload_selected_shooter_mod(engine, selected);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = status; }
            show_main_menu_page(engine, MainMenuPage::Mods);
        }
        _ => unreachable!("non-mods main-menu action routed to mods handler"),
    }
}
