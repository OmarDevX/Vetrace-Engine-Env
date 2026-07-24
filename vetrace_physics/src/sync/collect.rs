use super::*;

pub(super) type DynamicBodyEntry = (Entity, RigidBody3D, PhysicsEntityTransform);
pub(super) type StaticBodyEntry = (Entity, StaticBody, PhysicsEntityTransform);
pub(super) type KinematicBodyEntry = (Entity, KinematicBody, PhysicsEntityTransform);
pub(super) type ColliderEntry = (Entity, Collider, PhysicsEntityTransform);
pub(super) type MeshColliderEntry = (Entity, MeshCollider, PhysicsEntityTransform);

pub(super) struct PhysicsSyncSnapshot {
    pub(super) live_entities: HashSet<Entity>,
    pub(super) body_owner_entities: HashSet<Entity>,
    pub(super) collider_owner_entities: HashSet<Entity>,
    pub(super) dynamic_bodies: Vec<DynamicBodyEntry>,
    pub(super) static_bodies: Vec<StaticBodyEntry>,
    pub(super) kinematic_bodies: Vec<KinematicBodyEntry>,
    pub(super) colliders: Vec<ColliderEntry>,
    pub(super) mesh_colliders: Vec<MeshColliderEntry>,
    pub(super) velocities: Vec<(Entity, Vec3)>,
    pub(super) angular_velocities: Vec<(Entity, Vec3)>,
}

pub(super) fn collect_physics_sync_snapshot(engine: &Engine) -> PhysicsSyncSnapshot {
    let live_entities = engine.raw_world().entities().collect();
    let dynamic_bodies = engine
        .raw_world()
        .query::<RigidBody3D>()
        .into_iter()
        .map(|(entity, body)| (entity, body.clone(), physics_entity_transform(engine, entity)))
        .collect::<Vec<_>>();
    let static_bodies = engine
        .raw_world()
        .query::<StaticBody>()
        .into_iter()
        .map(|(entity, body)| (entity, body.clone(), physics_entity_transform(engine, entity)))
        .collect::<Vec<_>>();
    let kinematic_bodies = engine
        .raw_world()
        .query::<KinematicBody>()
        .into_iter()
        .map(|(entity, body)| (entity, body.clone(), physics_entity_transform(engine, entity)))
        .collect::<Vec<_>>();
    let colliders = engine
        .raw_world()
        .query::<Collider>()
        .into_iter()
        .map(|(entity, collider)| {
            (entity, collider.clone(), physics_entity_transform(engine, entity))
        })
        .collect::<Vec<_>>();
    let mesh_colliders = engine
        .raw_world()
        .query::<MeshCollider>()
        .into_iter()
        .map(|(entity, collider)| {
            (entity, collider.clone(), physics_entity_transform(engine, entity))
        })
        .collect::<Vec<_>>();
    let velocities = engine
        .raw_world()
        .query::<Velocity>()
        .into_iter()
        .map(|(entity, velocity)| (entity, velocity.linear))
        .collect();
    let angular_velocities = engine
        .raw_world()
        .query::<AngularVelocity>()
        .into_iter()
        .map(|(entity, velocity)| (entity, velocity.angular))
        .collect();

    let body_owner_entities = dynamic_bodies
        .iter()
        .map(|(entity, _, _)| *entity)
        .chain(static_bodies.iter().map(|(entity, _, _)| *entity))
        .chain(kinematic_bodies.iter().map(|(entity, _, _)| *entity))
        .collect();
    let collider_owner_entities = colliders
        .iter()
        .map(|(entity, _, _)| *entity)
        .chain(mesh_colliders.iter().map(|(entity, _, _)| *entity))
        .collect();

    PhysicsSyncSnapshot {
        live_entities,
        body_owner_entities,
        collider_owner_entities,
        dynamic_bodies,
        static_bodies,
        kinematic_bodies,
        colliders,
        mesh_colliders,
        velocities,
        angular_velocities,
    }
}
