use std::error::Error;
use std::path::PathBuf;

use vetrace_project::VetraceProject;
use vetrace_runtime::{RuntimeMode, VetraceRuntime};

fn main() -> Result<(), Box<dyn Error>> {
    let project_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("examples/lua_runtime_project"));
    let project = VetraceProject::load(project_path)?;
    let mut runtime = VetraceRuntime::load(project, RuntimeMode::StandaloneGame)?;
    runtime.run_until_stopped(None, 1.0 / 60.0)?;
    Ok(())
}
