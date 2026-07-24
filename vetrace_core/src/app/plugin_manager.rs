use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::error::Error;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::{Engine, Stage};

use super::Plugin;

#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self { Self::default() }

    pub fn add_plugin<P: Plugin + 'static>(&mut self, plugin: P) {
        self.add_boxed_plugin(Box::new(plugin));
    }

    pub fn add_boxed_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub fn initialize_plugins(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        self.sort_by_dependencies()?;
        for plugin in &mut self.plugins {
            let plugin_name = plugin.name();
            let started = Instant::now();
            plugin.initialize(engine)?;
            engine.profile_record_timing(&format!("plugin.{plugin_name}.initialize"), started.elapsed());
        }
        Ok(())
    }

    pub fn update_stage(&mut self, stage: Stage, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        for plugin in &mut self.plugins {
            if plugin.update_stage() != stage { continue; }
            let plugin_name = plugin.name();
            let started = Instant::now();
            plugin.update(engine, dt)?;
            engine.profile_record_timing(&format!("plugin.{plugin_name}.{stage:?}"), started.elapsed());
        }
        Ok(())
    }

    pub fn render_stage(&mut self, stage: Stage, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        for plugin in &mut self.plugins {
            if plugin.render_stage() != stage { continue; }
            let plugin_name = plugin.name();
            let started = Instant::now();
            plugin.render(engine)?;
            engine.profile_record_timing(&format!("plugin.{plugin_name}.{stage:?}"), started.elapsed());
        }
        Ok(())
    }

    /// Compatibility wrappers for integrations that drive plugins manually.
    pub fn update_plugins(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        self.update_stage(Stage::Update, engine, dt)
    }

    pub fn render_plugins(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        self.render_stage(Stage::Render, engine)
    }

    fn sort_by_dependencies(&mut self) -> Result<(), Box<dyn Error>> {
        let name_to_index: HashMap<&'static str, usize> = self
            .plugins
            .iter()
            .enumerate()
            .map(|(index, plugin)| (plugin.name(), index))
            .collect();
        let mut sorted = Vec::with_capacity(self.plugins.len());
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        fn visit(
            index: usize,
            plugins: &[Box<dyn Plugin>],
            name_to_index: &HashMap<&'static str, usize>,
            visiting: &mut HashSet<usize>,
            visited: &mut HashSet<usize>,
            sorted: &mut Vec<usize>,
        ) -> Result<(), Box<dyn Error>> {
            if visited.contains(&index) { return Ok(()); }
            if !visiting.insert(index) {
                return Err(format!("cyclic plugin dependency involving {}", plugins[index].name()).into());
            }
            for dependency in plugins[index].dependencies() {
                let Some(&dependency_index) = name_to_index.get(dependency) else {
                    return Err(format!("plugin {} depends on missing plugin {}", plugins[index].name(), dependency).into());
                };
                visit(dependency_index, plugins, name_to_index, visiting, visited, sorted)?;
            }
            visiting.remove(&index);
            visited.insert(index);
            sorted.push(index);
            Ok(())
        }

        for index in 0..self.plugins.len() {
            visit(index, &self.plugins, &name_to_index, &mut visiting, &mut visited, &mut sorted)?;
        }

        let mut old = std::mem::take(&mut self.plugins);
        let mut reordered = Vec::with_capacity(old.len());
        for index in sorted {
            reordered.push(std::mem::replace(&mut old[index], Box::new(EmptyPlugin)));
        }
        self.plugins = reordered;
        Ok(())
    }
}

struct EmptyPlugin;
impl Plugin for EmptyPlugin {
    fn name(&self) -> &'static str { "__empty__" }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
