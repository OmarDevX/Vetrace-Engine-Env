pub mod world;
pub mod entity;
pub mod component;

pub use world::World;            // ✅ From world.rs (where it's defined)
pub use entity::Entity;
pub use component::Component;
pub mod behaviour;
pub use behaviour::Behaviour;
