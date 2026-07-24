//! Deprecated compatibility launcher.
//!
//! Scene editing is implemented by `vetrace_editor` and hosted by
//! `vetrace_studio`. Keeping this executable as a thin launcher preserves old
//! scripts and desktop shortcuts without compiling a second editor stack.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "vetrace-map-builder is deprecated; launching Vetrace Studio's shared scene editor"
    );
    vetrace_studio::run_from_env()
}
