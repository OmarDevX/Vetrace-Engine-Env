mod builder;
mod error;
mod handle;

pub use builder::ActorBuilder;
pub use error::ActorError;
pub use handle::{Actor, ActorDestroyed};

pub(crate) use handle::{insert_actor_component, is_actor_managed_mutation, mark_transform_dirty};

#[cfg(test)]
mod tests;
