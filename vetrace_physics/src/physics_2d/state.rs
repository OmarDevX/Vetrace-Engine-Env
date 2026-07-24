use std::collections::BTreeMap;

use glam::Vec2;
use vetrace_core::Entity;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ActivePair2D {
    pub sensor: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PairKey2D(pub Entity, pub Entity);

impl PairKey2D {
    pub fn new(a: Entity, b: Entity) -> Self {
        if a <= b { Self(a, b) } else { Self(b, a) }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Physics2dStats {
    pub bodies: usize,
    pub broadphase_pairs: usize,
    pub contacts: usize,
    pub substeps: usize,
}

/// Runtime state for the optional built-in 2D solver.
pub struct Physics2dState {
    pub gravity: Vec2,
    pub solver_iterations: usize,
    pub max_substeps: usize,
    pub broadphase_cell_size: f32,
    pub stats: Physics2dStats,
    pub(crate) active_pairs: BTreeMap<PairKey2D, ActivePair2D>,
}

impl Physics2dState {
    pub fn new() -> Self {
        Self {
            gravity: Vec2::new(0.0, -9.81),
            solver_iterations: 4,
            max_substeps: 8,
            broadphase_cell_size: 2.0,
            stats: Physics2dStats::default(),
            active_pairs: BTreeMap::new(),
        }
    }
}

impl Default for Physics2dState {
    fn default() -> Self { Self::new() }
}
