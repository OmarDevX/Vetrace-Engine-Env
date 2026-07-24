use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use vetrace_core::InputState;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent};

#[derive(Debug)]
enum PendingInput {
    Key { code: String, down: bool },
    MouseButton { button: String, down: bool },
    PointerPosition { x: f32, y: f32 },
    PointerDelta { dx: f32, dy: f32 },
    Wheel { dx: f32, dy: f32 },
    Reset,
}

#[derive(Clone, Default)]
struct InputQueue(Rc<RefCell<VecDeque<PendingInput>>>);

impl InputQueue {
    fn push(&self, event: PendingInput) {
        self.0.borrow_mut().push_back(event);
    }

    fn drain_into(&self, input: &mut InputState) {
        input.begin_frame();
        for event in self.0.borrow_mut().drain(..) {
            match event {
                PendingInput::Key { code, down } => input.set_key_down(code, down),
                PendingInput::MouseButton { button, down } => {
                    input.set_mouse_button_down(button, down)
                }
                PendingInput::PointerPosition { x, y } => input.set_mouse_position(x, y),
                PendingInput::PointerDelta { dx, dy } => input.add_mouse_delta(dx, dy),
                PendingInput::Wheel { dx, dy } => input.add_mouse_wheel_delta(dx, dy),
                PendingInput::Reset => *input = InputState::new(),
            }
        }
    }
}

pub struct WebInputBridge {
    queue: InputQueue,
    _key_down: Closure<dyn FnMut(KeyboardEvent)>,
    _key_up: Closure<dyn FnMut(KeyboardEvent)>,
    _pointer_down: Closure<dyn FnMut(PointerEvent)>,
    _pointer_up: Closure<dyn FnMut(PointerEvent)>,
    _pointer_cancel: Closure<dyn FnMut(PointerEvent)>,
    _pointer_move: Closure<dyn FnMut(PointerEvent)>,
    _wheel: Closure<dyn FnMut(WheelEvent)>,
    _blur: Closure<dyn FnMut(Event)>,
}

impl WebInputBridge {
    pub fn attach(canvas: &HtmlCanvasElement) -> Result<Self, wasm_bindgen::JsValue> {
        canvas.set_tab_index(0);
        let queue = InputQueue::default();
        let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("window unavailable"))?;

        let key_queue = queue.clone();
        let key_down = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if should_prevent_key(&event.code()) {
                event.prevent_default();
            }
            key_queue.push(PendingInput::Key { code: event.code(), down: true });
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("keydown", key_down.as_ref().unchecked_ref())?;

        let key_queue = queue.clone();
        let key_up = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if should_prevent_key(&event.code()) {
                event.prevent_default();
            }
            key_queue.push(PendingInput::Key { code: event.code(), down: false });
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("keyup", key_up.as_ref().unchecked_ref())?;

        let pointer_queue = queue.clone();
        let focus_canvas = canvas.clone();
        let pointer_down = Closure::wrap(Box::new(move |event: PointerEvent| {
            event.prevent_default();
            let _ = focus_canvas.focus();
            let _ = focus_canvas.set_pointer_capture(event.pointer_id());
            pointer_queue.push(PendingInput::MouseButton {
                button: mouse_button_name(event.button()),
                down: true,
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("pointerdown", pointer_down.as_ref().unchecked_ref())?;

        let pointer_queue = queue.clone();
        let release_canvas = canvas.clone();
        let pointer_up = Closure::wrap(Box::new(move |event: PointerEvent| {
            event.prevent_default();
            let _ = release_canvas.release_pointer_capture(event.pointer_id());
            pointer_queue.push(PendingInput::MouseButton {
                button: mouse_button_name(event.button()),
                down: false,
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("pointerup", pointer_up.as_ref().unchecked_ref())?;

        let cancel_queue = queue.clone();
        let pointer_cancel = Closure::wrap(Box::new(move |_event: PointerEvent| {
            cancel_queue.push(PendingInput::Reset);
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(
            "pointercancel",
            pointer_cancel.as_ref().unchecked_ref(),
        )?;

        let pointer_queue = queue.clone();
        let pointer_move = Closure::wrap(Box::new(move |event: PointerEvent| {
            pointer_queue.push(PendingInput::PointerPosition {
                x: event.offset_x() as f32,
                y: event.offset_y() as f32,
            });
            pointer_queue.push(PendingInput::PointerDelta {
                dx: event.movement_x() as f32,
                dy: event.movement_y() as f32,
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("pointermove", pointer_move.as_ref().unchecked_ref())?;

        let wheel_queue = queue.clone();
        let wheel = Closure::wrap(Box::new(move |event: WheelEvent| {
            event.prevent_default();
            wheel_queue.push(PendingInput::Wheel {
                dx: event.delta_x() as f32,
                dy: event.delta_y() as f32,
            });
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("wheel", wheel.as_ref().unchecked_ref())?;

        let blur_queue = queue.clone();
        let blur = Closure::wrap(Box::new(move |_event: Event| {
            blur_queue.push(PendingInput::Reset);
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())?;

        Ok(Self {
            queue,
            _key_down: key_down,
            _key_up: key_up,
            _pointer_down: pointer_down,
            _pointer_up: pointer_up,
            _pointer_cancel: pointer_cancel,
            _pointer_move: pointer_move,
            _wheel: wheel,
            _blur: blur,
        })
    }

    pub fn begin_frame(&self, input: &mut InputState) {
        self.queue.drain_into(input);
    }
}

fn should_prevent_key(code: &str) -> bool {
    matches!(
        code,
        "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "Space"
    )
}

fn mouse_button_name(button: i16) -> String {
    match button {
        0 => "Left",
        1 => "Middle",
        2 => "Right",
        other => return format!("Button{other}"),
    }
    .to_string()
}
