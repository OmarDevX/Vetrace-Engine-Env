use std::any::Any;
use std::error::Error;

use crate::{Engine, Stage};

pub trait Plugin: Any {
    fn name(&self) -> &'static str;
    fn dependencies(&self) -> Vec<&'static str> { Vec::new() }
    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn Error>> { Ok(()) }
    fn update_stage(&self) -> Stage { Stage::Update }
    fn render_stage(&self) -> Stage { Stage::Render }
    fn update(&mut self, _engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> { Ok(()) }
    fn render(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn Error>> { Ok(()) }
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
