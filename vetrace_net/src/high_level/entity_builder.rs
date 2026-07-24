use vetrace_core::{Actor, Engine, Entity, World};

use crate::replication::{
    NetworkIdentity, ReplicatedComponentAdapter, ReplicatedComponentConfig,
    ReplicatedComponentList, ReplicationAuthority,
};

use super::{NetId, PeerId};

/// Builder used by games when an entity becomes networked.
///
/// This wires only identity/authority and a generic list of replicated
/// component configs. It does not know which concrete components exist.
pub struct NetworkEntityBuilder<'a> {
    world: &'a mut World,
    entity: Entity,
}

pub fn network_entity(world: &mut World, entity: Entity) -> NetworkEntityBuilder<'_> {
    NetworkEntityBuilder { world, entity }
}

impl<'a> NetworkEntityBuilder<'a> {
    pub fn identity(self, net_id: NetId) -> Self {
        self.world.insert(self.entity, NetworkIdentity {
            net_id,
            owner_id: None,
            authority: ReplicationAuthority::Server,
        });
        self
    }

    pub fn owner(self, owner_id: PeerId) -> Self {
        let identity = self.world.get::<NetworkIdentity>(self.entity).copied().unwrap_or(NetworkIdentity {
            net_id: owner_id,
            owner_id: None,
            authority: ReplicationAuthority::Server,
        });
        self.world.insert(self.entity, NetworkIdentity { owner_id: Some(owner_id), ..identity });
        self
    }

    pub fn authority(self, authority: ReplicationAuthority) -> Self {
        let identity = self.world.get::<NetworkIdentity>(self.entity).copied().unwrap_or(NetworkIdentity {
            net_id: self.entity.0,
            owner_id: None,
            authority,
        });
        self.world.insert(self.entity, NetworkIdentity { authority, ..identity });
        self
    }

    pub fn server_authoritative(self) -> Self {
        self.authority(ReplicationAuthority::Server)
    }

    pub fn client_authoritative(self, owner_id: PeerId) -> Self {
        self.owner(owner_id).authority(ReplicationAuthority::Client { owner_id })
    }

    /// Attach a replicated component config by adapter type.
    ///
    /// The adapter type lives outside `vetrace_net`, so this remains generic.
    pub fn replicate_component<C: ReplicatedComponentAdapter>(self, mut config: ReplicatedComponentConfig) -> Self {
        config.component_name = C::component_name().to_string();
        self.replicate_named_component(config)
    }

    /// Attach a replicated component config by explicit component name.
    pub fn replicate_named_component(self, config: ReplicatedComponentConfig) -> Self {
        let mut list = self.world.remove::<ReplicatedComponentList>(self.entity).unwrap_or_default();
        list.upsert(config);
        self.world.insert(self.entity, list);
        self
    }

    pub fn build(self) -> Entity {
        self.entity
    }
}


/// Actor-first variant of [`NetworkEntityBuilder`].
///
/// Games should prefer this surface so networking remains an Actor component
/// concern instead of requiring direct access to `Engine::world`.
pub struct NetworkActorBuilder<'a> {
    engine: &'a mut Engine,
    actor: Actor,
}

pub fn network_actor(engine: &mut Engine, actor: Actor) -> NetworkActorBuilder<'_> {
    NetworkActorBuilder { engine, actor }
}

impl<'a> NetworkActorBuilder<'a> {
    pub fn identity(self, net_id: NetId) -> Self {
        self.actor
            .insert(&mut *self.engine, NetworkIdentity {
                net_id,
                owner_id: None,
                authority: ReplicationAuthority::Server,
            })
            .expect("network actor must be alive");
        self
    }

    pub fn owner(self, owner_id: PeerId) -> Self {
        let identity = self
            .actor
            .get_component::<NetworkIdentity>(&*self.engine)
            .copied()
            .unwrap_or(NetworkIdentity {
                net_id: owner_id,
                owner_id: None,
                authority: ReplicationAuthority::Server,
            });
        self.actor
            .insert(&mut *self.engine, NetworkIdentity { owner_id: Some(owner_id), ..identity })
            .expect("network actor must be alive");
        self
    }

    pub fn authority(self, authority: ReplicationAuthority) -> Self {
        let identity = self
            .actor
            .get_component::<NetworkIdentity>(&*self.engine)
            .copied()
            .unwrap_or(NetworkIdentity {
                net_id: self.actor.entity().0,
                owner_id: None,
                authority,
            });
        self.actor
            .insert(&mut *self.engine, NetworkIdentity { authority, ..identity })
            .expect("network actor must be alive");
        self
    }

    pub fn server_authoritative(self) -> Self {
        self.authority(ReplicationAuthority::Server)
    }

    pub fn client_authoritative(self, owner_id: PeerId) -> Self {
        self.owner(owner_id).authority(ReplicationAuthority::Client { owner_id })
    }

    pub fn replicate_component<C: ReplicatedComponentAdapter>(self, mut config: ReplicatedComponentConfig) -> Self {
        config.component_name = C::component_name().to_string();
        self.replicate_named_component(config)
    }

    pub fn replicate_named_component(self, config: ReplicatedComponentConfig) -> Self {
        let mut list = self.actor.remove::<ReplicatedComponentList>(&mut *self.engine).unwrap_or_default();
        list.upsert(config);
        self.actor
            .insert(&mut *self.engine, list)
            .expect("network actor must be alive");
        self
    }

    pub fn build(self) -> Actor {
        self.actor
    }
}
