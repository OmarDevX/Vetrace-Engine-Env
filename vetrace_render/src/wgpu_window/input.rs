use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn pump_winit_input(
    engine: &mut Engine,
    event_loop: &mut EventLoop<()>,
    game_window_id: WindowId,
    game_window_focused: &mut bool,
    mut detached_profiler: Option<&mut DetachedProfilerWindow>,
) {
    if !engine.contains_resource::<InputState>() {
        engine.insert_resource(InputState::new());
    }
    let Some(input) = engine.get_resource_mut::<InputState>() else { return; };
    input.begin_frame();
    let _ = event_loop.pump_events(Some(Duration::ZERO), |event, target| {
        // This renderer is driven by the engine loop, not by winit's event
        // dispatch. `ControlFlow::Poll` prevents the event pump from parking
        // the thread until another mouse/window event arrives.
        target.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent { window_id, event } => {
                if window_id == game_window_id {
                    match event {
                        WindowEvent::CloseRequested => input.request_quit(),
                        WindowEvent::Focused(focused) => *game_window_focused = focused,
                        WindowEvent::KeyboardInput { event, .. } => {
                            let pressed = event.state == ElementState::Pressed;
                            if pressed {
                                if let Some(text) = event.text.as_ref() {
                                    input.push_text_input(text.as_ref());
                                }
                            }
                            if let PhysicalKey::Code(code) = event.physical_key {
                                if let Some(name) = key_name(code) {
                                    input.set_key_down(name, pressed);
                                }
                            }
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            if let Some(name) = mouse_button_name(button) {
                                input.set_mouse_button_down(name, state == ElementState::Pressed);
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => input.set_mouse_position(position.x as f32, position.y as f32),
                        WindowEvent::MouseWheel { delta, .. } => match delta {
                            MouseScrollDelta::LineDelta(x, y) => input.add_mouse_wheel_delta(x, y),
                            MouseScrollDelta::PixelDelta(pos) => input.add_mouse_wheel_delta(pos.x as f32, pos.y as f32),
                        },
                        _ => {}
                    }
                } else if let Some(_profiler) = detached_profiler.as_mut() {
                    #[cfg(all(feature = "egui_render", feature = "profiler"))]
                    _profiler.handle_window_event(window_id, &event);
                }
            }
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                if *game_window_focused {
                    input.add_mouse_delta(delta.0 as f32, delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}

pub(super) fn key_name(code: KeyCode) -> Option<&'static str> {
    Some(match code {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Digit0 => "Digit0",
        KeyCode::Digit1 => "Digit1",
        KeyCode::Digit2 => "Digit2",
        KeyCode::Digit3 => "Digit3",
        KeyCode::Digit4 => "Digit4",
        KeyCode::Digit5 => "Digit5",
        KeyCode::Digit6 => "Digit6",
        KeyCode::Digit7 => "Digit7",
        KeyCode::Digit8 => "Digit8",
        KeyCode::Digit9 => "Digit9",
        KeyCode::Minus => "Minus",
        KeyCode::Equal => "Equal",
        KeyCode::BracketLeft => "BracketLeft",
        KeyCode::BracketRight => "BracketRight",
        KeyCode::Enter | KeyCode::NumpadEnter => "Enter",
        KeyCode::F10 => "F10",
        KeyCode::ArrowUp => "ArrowUp",
        KeyCode::ArrowDown => "ArrowDown",
        KeyCode::ArrowLeft => "ArrowLeft",
        KeyCode::ArrowRight => "ArrowRight",
        KeyCode::Space => "Space",
        KeyCode::Tab => "Tab",
        KeyCode::Escape => "Escape",
        KeyCode::Delete => "Delete",
        KeyCode::Backspace => "Backspace",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "Control",
        KeyCode::AltLeft | KeyCode::AltRight => "Alt",
        _ => return None,
    })
}

pub(super) fn mouse_button_name(button: MouseButton) -> Option<&'static str> {
    Some(match button {
        MouseButton::Left => "Left",
        MouseButton::Right => "Right",
        MouseButton::Middle => "Middle",
        _ => return None,
    })
}
