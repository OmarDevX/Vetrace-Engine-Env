use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CStr;
use std::sync::{mpsc, Arc, Condvar, Mutex};

use mlua::{HookTriggers, Lua, Table, Value, VmState};

mod controller;
mod introspection;
mod protocol;

pub use controller::{LuaDebuggerController, LuaDebuggerHandle};
pub use protocol::*;

use introspection::*;

const DEBUG_SELF_REGISTRY_KEY: &str = "__vetrace_debug_self";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_breakpoint_path() {
        assert_eq!(normalize_path("@assets\\scripts\\player.lua"), "assets/scripts/player.lua");
    }

    #[test]
    fn error_line_parser_uses_first_numeric_segment() {
        assert_eq!(parse_error_line("assets/scripts/a.lua:27: bad value"), Some(27));
    }

    #[test]
    fn debugger_captures_active_lua_locals() {
        use std::time::Duration;

        let lua = Lua::new();
        let handle = LuaDebuggerController::install(&lua).expect("install debugger");
        let (controller, events) = handle.into_parts();
        let mut breakpoints = BTreeMap::new();
        breakpoints.insert(
            "assets/scripts/test.lua".to_owned(),
            BTreeSet::from([3usize]),
        );
        controller.apply(LuaDebuggerCommand::SetBreakpoints { breakpoints });
        controller.enter_callback("assets/scripts/test.lua", "update", Some(7));
        let instance = lua.create_table().expect("instance table");
        instance.set("health", 42).expect("instance value");
        controller
            .set_instance_table(&lua, &instance)
            .expect("debug instance table");

        let resume = controller.clone();
        let receiver = std::thread::spawn(move || {
            loop {
                match events.recv_timeout(Duration::from_secs(2)) {
                    Ok(LuaDebuggerEvent::Paused { state }) => {
                        resume.apply(LuaDebuggerCommand::Continue);
                        return state;
                    }
                    Ok(_) => continue,
                    Err(error) => {
                        resume.apply(LuaDebuggerCommand::Continue);
                        panic!("debugger did not pause: {error}");
                    }
                }
            }
        });

        let result = lua
            .load("local speed = 12\nspeed = speed + 1\nreturn speed")
            .set_name("@assets/scripts/test.lua")
            .eval::<i64>()
            .expect("execute debug script");
        assert_eq!(result, 13);
        let paused = receiver.join().expect("debug receiver");
        assert_eq!(paused.entity, Some(7));
        assert!(paused.locals.iter().any(|variable| {
            variable.name == "speed" && variable.value == LuaDebugValue::Integer(13)
        }));
        assert!(paused.locals.iter().any(|variable| variable.name == "self"));
        controller.clear_instance_table(&lua);
        controller.leave_callback();
    }
}
