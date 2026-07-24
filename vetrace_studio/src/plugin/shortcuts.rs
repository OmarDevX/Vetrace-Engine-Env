use super::*;

pub(super) fn append_keyboard_shortcuts(engine: &Engine, player_running: bool, commands: &mut Vec<StudioCommand>) {
    let Some(input) = engine.get_resource::<InputState>() else { return; };
    let control = input.is_key_down("Control") || input.is_key_down("Ctrl");
    let shift = input.is_key_down("Shift")
        || input.is_key_down("Left Shift")
        || input.is_key_down("Right Shift");
    if control && input.was_key_pressed("Z") {
        commands.push(if shift { StudioCommand::Redo } else { StudioCommand::Undo });
    }
    if control && input.was_key_pressed("S") {
        commands.push(StudioCommand::SaveScene);
    }
    if control && input.was_key_pressed("R") {
        commands.push(StudioCommand::ReloadScene);
    }
    if input.was_key_pressed("F6") {
        commands.push(if player_running {
            StudioCommand::StopProject
        } else {
            StudioCommand::PlayProject
        });
    }
}
