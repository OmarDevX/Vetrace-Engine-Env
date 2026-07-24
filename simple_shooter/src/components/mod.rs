//! Simple Shooter game-side components and configuration.
//!
//! Keep this module game-owned.  Renderer/physics crates expose neutral knobs;
//! Simple Shooter decides CLI names, demo defaults, and gameplay policy here.

pub mod config;
pub mod gameplay;
pub mod graphics_profile;
pub mod player;
pub mod ui;
pub mod visuals;
pub mod modding;
pub mod weapon;

pub use config::*;
pub use gameplay::*;
pub use graphics_profile::*;
pub use player::*;
pub use ui::*;
pub use visuals::*;
pub use modding::*;
pub use weapon::*;
