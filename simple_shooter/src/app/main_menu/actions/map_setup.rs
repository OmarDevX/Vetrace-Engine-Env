use super::*;

pub(super) fn apply(engine: &mut Engine, runtime: &mut ShooterRuntime, action: MainMenuAction) {
    match action {
        MainMenuAction::PreviousMap => {
            let count = map_count();
            set_map_validation_error(engine, String::new());
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                if count > 0 { menu.selected_map = (menu.selected_map + count - 1) % count; }
                menu.status = format!("Selected {}", map_name(menu.selected_map as u8));
            }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::NextMap => {
            let count = map_count();
            set_map_validation_error(engine, String::new());
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                if count > 0 { menu.selected_map = (menu.selected_map + 1) % count; }
                menu.status = format!("Selected {}", map_name(menu.selected_map as u8));
            }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::PreviousBotDifficulty => {
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                menu.selected_bot_difficulty = menu.selected_bot_difficulty.previous();
                menu.status = format!("Bot difficulty: {}", menu.selected_bot_difficulty.name());
            }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::NextBotDifficulty => {
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                menu.selected_bot_difficulty = menu.selected_bot_difficulty.next();
                menu.status = format!("Bot difficulty: {}", menu.selected_bot_difficulty.name());
            }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::BotCountDown => {
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.selected_bot_count = menu.selected_bot_count.saturating_sub(1); }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::BotCountUp => {
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.selected_bot_count = menu.selected_bot_count.saturating_add(1).min(MAX_BOT_COUNT); }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::MaxPlayersDown => {
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.selected_max_players = menu.selected_max_players.saturating_sub(1).max(1); }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        MainMenuAction::MaxPlayersUp => {
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.selected_max_players = menu.selected_max_players.saturating_add(1).min(MAX_PLAYER_LIMIT); }
            show_main_menu_page(engine, MainMenuPage::Maps);
        }
        _ => unreachable!("non-map_setup main-menu action routed to map_setup handler"),
    }
}
