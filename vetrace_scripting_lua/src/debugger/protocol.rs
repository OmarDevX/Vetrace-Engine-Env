use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum LuaDebuggerCommand {
    SetBreakpoints { breakpoints: BTreeMap<String, BTreeSet<usize>> },
    Pause,
    Continue,
    StepInto,
    StepOver,
    StepOut,
    SetWatches { expressions: Vec<String> },
    SetBreakOnError { enabled: bool },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LuaDebugVariable {
    pub name: String,
    pub value: LuaDebugValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum LuaDebugValue {
    Nil,
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Table(Vec<LuaDebugVariable>),
    Function,
    UserData(String),
    Thread,
    Error(String),
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaStackFrame {
    pub name: String,
    pub source: String,
    pub line: Option<usize>,
    pub defined_line: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LuaPausedState {
    pub reason: String,
    pub path: String,
    pub line: usize,
    pub callback: String,
    pub entity: Option<u64>,
    pub stack: Vec<LuaStackFrame>,
    pub locals: Vec<LuaDebugVariable>,
    pub watches: Vec<LuaDebugVariable>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum LuaDebuggerEvent {
    Ready,
    Paused { state: LuaPausedState },
    Resumed,
    Error {
        path: String,
        line: Option<usize>,
        callback: String,
        entity: Option<u64>,
        message: String,
    },
}
