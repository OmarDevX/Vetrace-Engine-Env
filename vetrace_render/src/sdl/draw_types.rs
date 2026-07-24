use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawFace {
    pub(crate) points: [Vec2; 4],
    pub(crate) depth: f32,
    pub(crate) color: Color,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DrawWire {
    pub(crate) points: [Vec2; 8],
    pub(crate) depth: f32,
    pub(crate) color: Color,
}
