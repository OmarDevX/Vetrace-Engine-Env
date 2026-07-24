use vetrace_core::Entity;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LuaDiagnosticTarget {
    Autoload(String),
    Entity { entity: Entity, script: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaScriptError {
    pub target: LuaDiagnosticTarget,
    pub callback: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct LuaDiagnostics {
    errors: Vec<LuaScriptError>,
}

impl LuaDiagnostics {
    pub fn errors(&self) -> &[LuaScriptError] { &self.errors }
    pub fn clear(&mut self) { self.errors.clear(); }
    pub fn push(&mut self, error: LuaScriptError) { self.errors.push(error); }
}

#[derive(Clone, Copy, Debug)]
pub struct LuaRuntimeConfig {
    pub fail_fast: bool,
    pub max_errors_per_frame: u32,
}

impl Default for LuaRuntimeConfig {
    fn default() -> Self {
        Self { fail_fast: false, max_errors_per_frame: 16 }
    }
}
