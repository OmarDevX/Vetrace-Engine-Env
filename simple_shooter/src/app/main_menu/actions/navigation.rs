use super::*;

pub(super) fn apply(engine: &mut Engine, runtime: &mut ShooterRuntime, action: MainMenuAction) {
    match action {
        MainMenuAction::RandomizePlayer => randomize_main_menu_player(engine),
        MainMenuAction::Maps => show_main_menu_page(engine, MainMenuPage::Maps),
        MainMenuAction::Shop => show_main_menu_page(engine, MainMenuPage::Shop),
        MainMenuAction::Mods => show_main_menu_page(engine, MainMenuPage::Mods),
        MainMenuAction::Settings => show_main_menu_page(engine, MainMenuPage::Settings),
        MainMenuAction::ClosePage => show_main_menu_page(engine, MainMenuPage::Home),
        _ => unreachable!("non-navigation main-menu action routed to navigation handler"),
    }
}
