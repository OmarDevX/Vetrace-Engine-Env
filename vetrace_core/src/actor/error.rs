use std::error::Error;
use std::fmt;

use crate::{ActorId, Entity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorError {
    DeadActor(Entity),
    DeadParent(Entity),
    CannotParentToSelf(Entity),
    HierarchyCycle { actor: Entity, parent: Entity },
    DuplicateActorId(ActorId),
    ManagedComponent(&'static str),
}

impl fmt::Display for ActorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActorError::DeadActor(entity) => write!(formatter, "actor {:?} is not alive", entity),
            ActorError::DeadParent(entity) => write!(formatter, "parent actor {:?} is not alive", entity),
            ActorError::CannotParentToSelf(entity) => write!(formatter, "actor {:?} cannot be its own parent", entity),
            ActorError::HierarchyCycle { actor, parent } => {
                write!(formatter, "parenting actor {:?} under {:?} would create a hierarchy cycle", actor, parent)
            }
            ActorError::DuplicateActorId(id) => write!(formatter, "actor ID {id} is already in use"),
            ActorError::ManagedComponent(name) => write!(formatter, "{name} is managed by a dedicated Actor API"),
        }
    }
}

impl Error for ActorError {}
