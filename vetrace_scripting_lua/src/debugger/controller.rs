use super::*;

#[derive(Clone, Debug)]
struct DebugContext {
    path: String,
    callback: String,
    entity: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunMode {
    Continue,
    Paused,
    StepInto,
    StepOver { depth: usize },
    StepOut { depth: usize },
}

struct DebuggerState {
    breakpoints: BTreeMap<String, BTreeSet<usize>>,
    watches: Vec<String>,
    break_on_error: bool,
    pause_requested: bool,
    run_mode: RunMode,
    context: Option<DebugContext>,
    last_location: Option<(String, usize, usize)>,
}

impl Default for DebuggerState {
    fn default() -> Self {
        Self {
            breakpoints: BTreeMap::new(),
            watches: Vec::new(),
            break_on_error: true,
            pause_requested: false,
            run_mode: RunMode::Continue,
            context: None,
            last_location: None,
        }
    }
}

struct DebuggerShared {
    state: Mutex<DebuggerState>,
    wake: Condvar,
    events: mpsc::Sender<LuaDebuggerEvent>,
}

#[derive(Clone)]
pub struct LuaDebuggerController {
    shared: Arc<DebuggerShared>,
}

pub struct LuaDebuggerHandle {
    controller: LuaDebuggerController,
    events: mpsc::Receiver<LuaDebuggerEvent>,
}

impl LuaDebuggerHandle {
    pub fn controller(&self) -> LuaDebuggerController { self.controller.clone() }

    pub fn recv(&self) -> Result<LuaDebuggerEvent, mpsc::RecvError> { self.events.recv() }

    pub fn try_recv(&self) -> Result<LuaDebuggerEvent, mpsc::TryRecvError> {
        self.events.try_recv()
    }

    pub fn into_parts(self) -> (LuaDebuggerController, mpsc::Receiver<LuaDebuggerEvent>) {
        (self.controller, self.events)
    }
}

impl LuaDebuggerController {
    pub fn install(lua: &Lua) -> mlua::Result<LuaDebuggerHandle> {
        let (event_sender, event_receiver) = mpsc::channel();
        let controller = Self {
            shared: Arc::new(DebuggerShared {
                state: Mutex::new(DebuggerState::default()),
                wake: Condvar::new(),
                events: event_sender,
            }),
        };
        let hook_controller = controller.clone();
        lua.set_hook(HookTriggers::EVERY_LINE, move |lua, debug| {
            let Some(line) = debug.current_line() else {
                return Ok(VmState::Continue);
            };
            hook_controller.on_line(lua, line);
            Ok(VmState::Continue)
        })?;
        let _ = controller.shared.events.send(LuaDebuggerEvent::Ready);
        Ok(LuaDebuggerHandle {
            controller,
            events: event_receiver,
        })
    }

    pub fn apply(&self, command: LuaDebuggerCommand) {
        let mut state = self.shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match command {
            LuaDebuggerCommand::SetBreakpoints { breakpoints } => {
                state.breakpoints = breakpoints
                    .into_iter()
                    .map(|(path, lines)| (normalize_path(&path), lines))
                    .collect();
            }
            LuaDebuggerCommand::Pause => state.pause_requested = true,
            LuaDebuggerCommand::Continue => {
                state.pause_requested = false;
                state.run_mode = RunMode::Continue;
                state.last_location = None;
                self.shared.wake.notify_all();
                let _ = self.shared.events.send(LuaDebuggerEvent::Resumed);
            }
            LuaDebuggerCommand::StepInto => {
                state.pause_requested = false;
                state.run_mode = RunMode::StepInto;
                self.shared.wake.notify_all();
                let _ = self.shared.events.send(LuaDebuggerEvent::Resumed);
            }
            LuaDebuggerCommand::StepOver => {
                let depth = state.last_location.as_ref().map(|(_, _, depth)| *depth).unwrap_or(0);
                state.pause_requested = false;
                state.run_mode = RunMode::StepOver { depth };
                self.shared.wake.notify_all();
                let _ = self.shared.events.send(LuaDebuggerEvent::Resumed);
            }
            LuaDebuggerCommand::StepOut => {
                let depth = state.last_location.as_ref().map(|(_, _, depth)| *depth).unwrap_or(0);
                state.pause_requested = false;
                state.run_mode = RunMode::StepOut { depth };
                self.shared.wake.notify_all();
                let _ = self.shared.events.send(LuaDebuggerEvent::Resumed);
            }
            LuaDebuggerCommand::SetWatches { expressions } => {
                state.watches = expressions
                    .into_iter()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty())
                    .collect();
            }
            LuaDebuggerCommand::SetBreakOnError { enabled } => state.break_on_error = enabled,
        }
    }

    pub(crate) fn enter_callback(&self, path: &str, callback: &str, entity: Option<u64>) {
        let mut state = self.shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.context = Some(DebugContext {
            path: normalize_path(path),
            callback: callback.to_owned(),
            entity,
        });
    }

    pub(crate) fn leave_callback(&self) {
        let mut state = self.shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.context = None;
        state.last_location = None;
    }

    pub(crate) fn set_instance_table(&self, lua: &Lua, table: &Table) -> mlua::Result<()> {
        lua.set_named_registry_value(DEBUG_SELF_REGISTRY_KEY, table.clone())
    }

    pub(crate) fn clear_instance_table(&self, lua: &Lua) {
        let _ = lua.unset_named_registry_value(DEBUG_SELF_REGISTRY_KEY);
    }

    pub(crate) fn report_error(&self, lua: &Lua, message: &str) {
        let (context, break_on_error, watches) = {
            let state = self.shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            (state.context.clone(), state.break_on_error, state.watches.clone())
        };
        let context = context.unwrap_or(DebugContext {
            path: "<unknown>".to_owned(),
            callback: "<unknown>".to_owned(),
            entity: None,
        });
        let line = parse_error_line(message);
        let _ = self.shared.events.send(LuaDebuggerEvent::Error {
            path: context.path.clone(),
            line,
            callback: context.callback.clone(),
            entity: context.entity,
            message: message.to_owned(),
        });
        if !break_on_error { return; }

        let line = line.unwrap_or(1);
        let local_table = collect_local_table(lua);
        let paused = LuaPausedState {
            reason: "error".to_owned(),
            path: context.path,
            line,
            callback: context.callback,
            entity: context.entity,
            stack: collect_stack(lua),
            locals: collect_debug_variables(lua, local_table.as_ref()),
            watches: evaluate_watches(lua, &watches, local_table.as_ref()),
        };
        self.pause_with_state(paused, 0);
    }

    fn on_line(&self, lua: &Lua, line: usize) {
        let depth = stack_depth(lua);
        let (context, reason, watches) = {
            let mut state = self.shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(context) = state.context.clone() else { return; };
            let location_changed = state
                .last_location
                .as_ref()
                .map_or(true, |(path, previous_line, previous_depth)| {
                    path != &context.path || *previous_line != line || *previous_depth != depth
                });
            let breakpoint = state
                .breakpoints
                .get(&context.path)
                .is_some_and(|lines| lines.contains(&line));
            let reason = if state.pause_requested {
                Some("pause")
            } else if breakpoint {
                Some("breakpoint")
            } else {
                match state.run_mode {
                    RunMode::Continue | RunMode::Paused => None,
                    RunMode::StepInto if location_changed => Some("step"),
                    RunMode::StepOver { depth: target } if location_changed && depth <= target => Some("step"),
                    RunMode::StepOut { depth: target } if location_changed && depth < target => Some("step"),
                    _ => None,
                }
            };
            state.last_location = Some((context.path.clone(), line, depth));
            let Some(reason) = reason else { return; };
            state.pause_requested = false;
            state.run_mode = RunMode::Paused;
            (context, reason.to_owned(), state.watches.clone())
        };

        let local_table = collect_local_table(lua);
        let paused = LuaPausedState {
            reason,
            path: context.path,
            line,
            callback: context.callback,
            entity: context.entity,
            stack: collect_stack(lua),
            locals: collect_debug_variables(lua, local_table.as_ref()),
            watches: evaluate_watches(lua, &watches, local_table.as_ref()),
        };
        self.pause_with_state(paused, depth);
    }

    fn pause_with_state(&self, paused: LuaPausedState, depth: usize) {
        let _ = self.shared.events.send(LuaDebuggerEvent::Paused { state: paused });
        let mut state = self.shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.last_location = state
            .last_location
            .take()
            .map(|(path, line, _)| (path, line, depth));
        while state.run_mode == RunMode::Paused {
            state = self
                .shared
                .wake
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}
