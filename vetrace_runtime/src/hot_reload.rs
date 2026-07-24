use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use vetrace_core::{Engine, Plugin, Stage};
use vetrace_project::{ProjectPath, VetraceProject};
use vetrace_scripting_lua::{reload_script_from_file_as, LuaScriptingState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileStamp {
    modified: Option<SystemTime>,
    length: u64,
    content_hash: u64,
}

/// Lightweight project-script hot reload used by the generic player.
///
/// The plugin polls only `assets/scripts/**/*.lua`. A replacement is compiled
/// before live instances are restarted, so an invalid saved file never
/// replaces the last running valid template.
pub struct LuaProjectHotReloadPlugin {
    project: VetraceProject,
    known: BTreeMap<ProjectPath, FileStamp>,
    elapsed: f32,
    interval: f32,
}

impl LuaProjectHotReloadPlugin {
    pub fn new(project: VetraceProject) -> Self {
        Self { project, known: BTreeMap::new(), elapsed: 0.0, interval: 0.25 }
    }

    pub fn with_interval(mut self, seconds: f32) -> Self {
        if seconds.is_finite() && seconds > 0.0 { self.interval = seconds; }
        self
    }

    fn scan(&self) -> Vec<(ProjectPath, PathBuf, FileStamp)> {
        let mut files = Vec::new();
        collect_lua_files(self.project.paths().scripts(), &mut files);
        files.into_iter().filter_map(|path| {
            let metadata = fs::metadata(&path).ok()?;
            let project_path = self.project.paths().to_project_path(&path).ok()?;
            let content_hash = fs::read(&path).ok().map(|bytes| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut hasher);
                hasher.finish()
            }).unwrap_or_default();
            Some((project_path, path, FileStamp {
                modified: metadata.modified().ok(),
                length: metadata.len(),
                content_hash,
            }))
        }).collect()
    }
}

impl Plugin for LuaProjectHotReloadPlugin {
    fn name(&self) -> &'static str { "lua_project_hot_reload" }
    fn dependencies(&self) -> Vec<&'static str> { vec!["lua_scripting"] }
    fn update_stage(&self) -> Stage { Stage::PostUpdate }

    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        self.known = self.scan().into_iter().map(|(path, _, stamp)| (path, stamp)).collect();
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        self.elapsed += dt.max(0.0).min(0.1);
        if self.elapsed < self.interval { return Ok(()); }
        self.elapsed = 0.0;

        let scanned = self.scan();
        let current = scanned.iter().map(|(path, _, stamp)| (path.clone(), *stamp)).collect::<BTreeMap<_, _>>();
        let active_before = active_script_keys(engine);
        let mut reloaded = BTreeSet::new();
        let mut helper_changed = false;

        for (project_path, filesystem_path, stamp) in scanned {
            if self.known.get(&project_path).is_some_and(|known| known == &stamp) { continue; }
            let key = project_path.as_str().to_owned();
            match reload_script_from_file_as(engine, &filesystem_path, key.clone()) {
                Ok(_) => {
                    println!("VETRACE_SCRIPT_RELOADED\t{}", project_path.as_str());
                    helper_changed |= !active_before.contains(&key);
                    reloaded.insert(key);
                }
                Err(error) => report_lua_diagnostic(project_path.as_str(), &error.to_string()),
            }
        }

        // Project modules are evaluated inside the environment of the script
        // that requires them. Restart active entry scripts after a helper file
        // changes so their per-environment module cache is rebuilt safely.
        if helper_changed {
            for (key, path) in active_script_paths(engine) {
                if reloaded.contains(&key) { continue; }
                match reload_script_from_file_as(engine, &path, key.clone()) {
                    Ok(_) => println!("VETRACE_SCRIPT_RELOADED\t{key}"),
                    Err(error) => report_lua_diagnostic(&key, &error.to_string()),
                }
            }
        }

        self.known = current;
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

fn active_script_keys(engine: &Engine) -> BTreeSet<String> {
    let Some(state) = engine.get_resource::<LuaScriptingState>() else { return BTreeSet::new(); };
    state
        .autoload_scripts
        .iter()
        .cloned()
        .chain(state.entity_scripts.values().cloned())
        .collect()
}

fn active_script_paths(engine: &Engine) -> Vec<(String, PathBuf)> {
    let Some(state) = engine.get_resource::<LuaScriptingState>() else { return Vec::new(); };
    active_script_keys(engine)
        .into_iter()
        .filter_map(|key| {
            state
                .scripts
                .get(&key)
                .and_then(|script| script.meta.path.clone())
                .map(|path| (key, path))
        })
        .collect()
}

fn report_lua_diagnostic(path: &str, error: &str) {
    let (line, column, message) = parse_lua_error(error);
    eprintln!(
        "VETRACE_SCRIPT_DIAGNOSTIC\t{}\t{}\t{}\t{}",
        path,
        line,
        column,
        message.replace('\n', " "),
    );
}

fn collect_lua_files(directory: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else { return; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else { continue; };
        if file_type.is_symlink() { continue; }
        if file_type.is_dir() {
            collect_lua_files(&path, output);
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("lua"))
        {
            output.push(path);
        }
    }
}

fn parse_lua_error(error: &str) -> (usize, usize, String) {
    let parts = error.split(':').collect::<Vec<_>>();
    for index in 0..parts.len() {
        let Ok(line) = parts[index].trim().parse::<usize>() else { continue; };
        let mut next = index + 1;
        let column = parts.get(next)
            .and_then(|part| part.trim().parse::<usize>().ok())
            .map(|column| { next += 1; column })
            .unwrap_or(1);
        let message = parts[next..].join(":").trim().to_owned();
        return (line.max(1), column.max(1), if message.is_empty() { error.to_owned() } else { message });
    }
    (1, 1, error.to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_lua_error;

    #[test]
    fn parses_lua_line_numbers() {
        let (line, column, message) = parse_lua_error("player.lua:12:4: unexpected symbol");
        assert_eq!((line, column), (12, 4));
        assert_eq!(message, "unexpected symbol");
    }
}
