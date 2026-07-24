use std::error::Error;

use crate::Engine;

pub trait App {
    fn setup(&mut self, _engine: &mut Engine) {}

    /// Fallible startup hook for product runtimes and tools that need to report
    /// scene/project loading failures. Existing apps can keep implementing
    /// `setup`; the default forwards to it.
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        self.setup(engine);
        Ok(())
    }

    fn update(&mut self, _engine: &mut Engine, _dt: f32) {}
    fn render(&mut self, _engine: &mut Engine) {}
    fn shutdown(&mut self, _engine: &mut Engine) {}
}
