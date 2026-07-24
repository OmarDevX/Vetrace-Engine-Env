use crate::{Actor, ActorError, Component, Engine};

/// A reusable set of components that can be attached to an actor atomically.
///
/// Subsystems should define their own bundles instead of adding render/physics
/// types to `vetrace_core`.
pub trait Bundle: 'static {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError>;
}

macro_rules! tuple_bundle {
    ($($name:ident),+) => {
        impl<$($name: Component),+> Bundle for ($($name,)+) {
            #[allow(non_snake_case)]
            fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
                let ($($name,)+) = self;
                $(actor.insert(engine, $name)?;)+
                Ok(())
            }
        }
    };
}

tuple_bundle!(A);
tuple_bundle!(A, B);
tuple_bundle!(A, B, C);
tuple_bundle!(A, B, C, D);
tuple_bundle!(A, B, C, D, E);
tuple_bundle!(A, B, C, D, E, F);
tuple_bundle!(A, B, C, D, E, F, G);
tuple_bundle!(A, B, C, D, E, F, G, H);
