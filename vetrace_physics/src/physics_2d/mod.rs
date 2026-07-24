//! Compile-time optional 2D rigid-body and collision plugin.
//!
//! The entire module is excluded unless `vetrace_physics/physics_2d` is enabled.
//! It intentionally does not depend on the renderer and can be used headlessly.

mod actor_ext;
mod components;
mod events;
mod geometry;
mod plugin;
mod queries;
mod solver;
mod state;
mod transforms;

pub use actor_ext::{Physics2dActorExt, RigidBody2dBundle};
pub use components::{BodyType2D, Collider2D, ColliderShape2D, RigidBody2D, Velocity2D};
pub use events::{CollisionContact2D, CollisionStarted2D, CollisionStopped2D};
pub use plugin::Physics2dPlugin;
pub use queries::{
    overlap_box_2d, overlap_circle_2d, point_query_2d, raycast_2d, Physics2dQueryFilter,
    RaycastHit2D,
};
pub use state::{Physics2dState, Physics2dStats};
