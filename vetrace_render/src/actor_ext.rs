use vetrace_core::{Actor, ActorError, Bundle, Engine};

use crate::{Material, Renderable, Shape};

#[derive(Clone, Debug)]
pub struct RenderBundle {
    pub shape: Shape,
    pub material: Material,
    pub renderable: Renderable,
}

impl Bundle for RenderBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
        actor.insert(engine, self.shape)?;
        actor.insert(engine, self.material)?;
        actor.insert(engine, self.renderable)?;
        Ok(())
    }
}

pub trait RenderActorExt {
    fn set_visible(self, engine: &mut Engine, visible: bool) -> Result<(), ActorError>;
    fn set_material(self, engine: &mut Engine, material: Material) -> Result<(), ActorError>;
}

impl RenderActorExt for Actor {
    fn set_visible(self, engine: &mut Engine, visible: bool) -> Result<(), ActorError> {
        self.ensure_renderable(engine)?;
        if let Some(renderable) = self.get_component_mut::<Renderable>(engine) {
            renderable.visible = visible;
        }
        Ok(())
    }

    fn set_material(self, engine: &mut Engine, material: Material) -> Result<(), ActorError> {
        self.insert(engine, material)
    }
}

trait EnsureRenderable {
    fn ensure_renderable(self, engine: &mut Engine) -> Result<(), ActorError>;
}

impl EnsureRenderable for Actor {
    fn ensure_renderable(self, engine: &mut Engine) -> Result<(), ActorError> {
        if !self.has::<Renderable>(engine) { self.insert(engine, Renderable::default())?; }
        Ok(())
    }
}

#[cfg(feature = "render_2d")]
#[derive(Clone, Debug)]
pub struct Sprite2DBundle {
    pub sprite: crate::Sprite2D,
    pub canvas: crate::CanvasItem2D,
}

#[cfg(feature = "render_2d")]
impl Bundle for Sprite2DBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
        actor.insert(engine, self.sprite)?;
        actor.insert(engine, self.canvas)?;
        Ok(())
    }
}
