use super::*;

pub(crate) fn pump_sdl_input(engine: &mut Engine, _event_pump: &mut EventPump) {
    if !engine.contains_resource::<InputState>() {
        engine.insert_resource(InputState::new());
    }
    let Some(input) = engine.get_resource_mut::<InputState>() else { return; };
    input.begin_frame();

    // Do not use `EventPump::poll_iter()` here. SDL can emit newer event IDs
    // than the `sdl2` Rust crate knows how to convert, especially on newer
    // Linux/Wayland/XKB stacks. `poll_iter()` panics while constructing
    // `Event` for those IDs. Raw polling lets the render/input bridge handle
    // the stable events it cares about and safely ignore unknown ones.
    unsafe {
        let mut raw = MaybeUninit::<sys::SDL_Event>::uninit();
        while sys::SDL_PollEvent(raw.as_mut_ptr()) != 0 {
            let event = raw.assume_init();
            match event.type_ {
                t if t == sys::SDL_EventType::SDL_QUIT as u32 => input.request_quit(),
                t if t == sys::SDL_EventType::SDL_KEYDOWN as u32 => {
                    let key = event.key;
                    if key.repeat == 0 {
                        input.set_key_down(raw_key_name(key.keysym.sym), true);
                    }
                }
                t if t == sys::SDL_EventType::SDL_KEYUP as u32 => {
                    let key = event.key;
                    input.set_key_down(raw_key_name(key.keysym.sym), false);
                }
                t if t == sys::SDL_EventType::SDL_MOUSEBUTTONDOWN as u32 => {
                    let button = event.button;
                    input.set_mouse_button_down(raw_mouse_button_name(button.button), true);
                }
                t if t == sys::SDL_EventType::SDL_MOUSEBUTTONUP as u32 => {
                    let button = event.button;
                    input.set_mouse_button_down(raw_mouse_button_name(button.button), false);
                }
                t if t == sys::SDL_EventType::SDL_MOUSEMOTION as u32 => {
                    let motion = event.motion;
                    input.set_mouse_position(motion.x as f32, motion.y as f32);
                    input.add_mouse_delta(motion.xrel as f32, motion.yrel as f32);
                }
                t if t == sys::SDL_EventType::SDL_MOUSEWHEEL as u32 => {
                    let wheel = event.wheel;
                    input.add_mouse_wheel_delta(wheel.x as f32, wheel.y as f32);
                }
                _ => {
                    // Unknown/new SDL events are intentionally ignored.
                }
            }
        }
    }
}

fn raw_key_name(key: sys::SDL_Keycode) -> String {
    match key {
        119 => "W".to_string(),       // SDLK_w
        97 => "A".to_string(),        // SDLK_a
        115 => "S".to_string(),       // SDLK_s
        100 => "D".to_string(),       // SDLK_d
        32 => "Space".to_string(),    // SDLK_SPACE
        27 => "Escape".to_string(),   // SDLK_ESCAPE
        9 => "Tab".to_string(),       // SDLK_TAB
        1073741897 => "F10".to_string(), // SDLK_F10
        1073742049 | 1073742053 => "Shift".to_string(), // SDLK_LSHIFT/RSHIFT
        1073742048 | 1073742052 => "Ctrl".to_string(),  // SDLK_LCTRL/RCTRL
        other => format!("SDLK_{other}"),
    }
}

fn raw_mouse_button_name(button: u8) -> String {
    match button {
        1 => "Left".to_string(),
        2 => "Middle".to_string(),
        3 => "Right".to_string(),
        4 => "X1".to_string(),
        5 => "X2".to_string(),
        other => format!("Button{other}"),
    }
}
