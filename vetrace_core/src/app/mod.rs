mod app;
mod builder;
mod frame_pacing;
mod plugin_manager;
mod plugin_trait;
mod runner;

pub use app::App;
pub use builder::AppBuilder;
pub use plugin_manager::PluginManager;
pub use plugin_trait::Plugin;
pub use runner::AppRunner;

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::Engine;

    use super::*;

    #[derive(Default)]
    struct CountingApp {
        initialized: bool,
        updates: usize,
        shutdown: bool,
    }

    impl App for CountingApp {
        fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn Error>> {
            self.initialized = true;
            Ok(())
        }

        fn update(&mut self, engine: &mut Engine, _dt: f32) {
            self.updates += 1;
            if self.updates == 2 { engine.stop(); }
        }

        fn shutdown(&mut self, _engine: &mut Engine) {
            self.shutdown = true;
        }
    }

    #[test]
    fn app_runner_exposes_controlled_lifecycle() {
        let mut runner = AppBuilder::new().build(CountingApp::default());
        runner.initialize().unwrap();
        assert!(runner.app().initialized);
        runner.run_frames(10, 1.0 / 60.0).unwrap();
        assert_eq!(runner.app().updates, 2);
        assert!(runner.app().shutdown);
        assert_eq!(runner.frame_count(), 2);
    }
}
