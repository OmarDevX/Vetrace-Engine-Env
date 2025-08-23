use std::{env, fs, io::Write, path::{Path, PathBuf}};

fn main() {
    println!("cargo:rerun-if-changed=generated/components");
    if let Ok(ws) = env::var("CARGO_WORKSPACE_DIR") {
        println!("cargo:rerun-if-changed={}/generated/components", ws);
    }
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_components.rs");
    let mut file = std::fs::File::create(&dest_path).unwrap();
    writeln!(file, "use crate::engine::engine::Engine;").unwrap();

    let mut files: Vec<(String, PathBuf)> = Vec::new();
    let mut collect = |dir: &Path| {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        files.push((stem.to_string(), path.clone()));
                    }
                }
            }
        }
    };

    collect(Path::new("generated/components"));
    if let Ok(ws) = env::var("CARGO_WORKSPACE_DIR") {
        let p = Path::new(&ws).join("generated/components");
        if p.exists() {
            collect(&p);
        }
    }

    for (name, path) in &files {
        let mod_name = name.to_lowercase();
        writeln!(file, "pub mod {mod_name} {{").unwrap();
        writeln!(file, "    include!(\"{}\");", path.display()).unwrap();
        writeln!(file, "}}").unwrap();
        writeln!(file, "pub use {mod_name}::{name};").unwrap();
    }

    writeln!(file, "pub fn register_generated_components(engine: &mut Engine) {{").unwrap();
    for (name, _) in &files {
        let mod_name = name.to_lowercase();
        writeln!(file, "    engine.auto_register_component::<{mod_name}::{name}>(\"{name}\");").unwrap();
    }
    writeln!(file, "}}").unwrap();
}
