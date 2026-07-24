use crate::{ActorError, Engine, GlobalTransform, Parent, Transform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Health(u32);

#[test]
fn actor_spawns_with_core_components_and_custom_data() {
    let mut engine = Engine::new();
    let actor = engine
        .spawn_actor("Player")
        .with(Transform { translation: glam::Vec3::new(1.0, 2.0, 3.0), ..Transform::default() })
        .with(Health(100))
        .tag("player")
        .source("test")
        .build();

    assert_eq!(actor.name(&engine), Some("Player"));
    assert!(actor.has::<Transform>(&engine));
    assert!(actor.has::<GlobalTransform>(&engine));
    assert_eq!(actor.component::<Health>(&engine).copied(), Some(Health(100)));
    assert!(actor.has_tag(&engine, "player"));
    assert_eq!(actor.source(&engine), Some("test"));
    assert_eq!(
        actor.global_transform(&engine).map(|transform| transform.translation),
        Some(glam::Vec3::new(1.0, 2.0, 3.0)),
    );
}

#[test]
fn parented_transform_updates_global_immediately() {
    let mut engine = Engine::new();
    let root = engine
        .spawn_actor("Root")
        .with(Transform { translation: glam::Vec3::new(10.0, 0.0, 0.0), ..Transform::default() })
        .build();
    let child = engine
        .spawn_actor("Child")
        .with(Transform { translation: glam::Vec3::new(2.0, 0.0, 0.0), ..Transform::default() })
        .child_of(root)
        .unwrap()
        .build();

    assert_eq!(
        child.global_transform(&engine).map(|transform| transform.translation),
        Some(glam::Vec3::new(12.0, 0.0, 0.0)),
    );

    child
        .insert(
            &mut engine,
            Transform { translation: glam::Vec3::new(3.0, 0.0, 0.0), ..Transform::default() },
        )
        .unwrap();
    assert_eq!(
        child.global_transform(&engine).map(|transform| transform.translation),
        Some(glam::Vec3::new(13.0, 0.0, 0.0)),
    );
}

#[test]
fn hierarchy_is_bidirectional_and_despawns_recursively() {
    let mut engine = Engine::new();
    let root = engine.spawn_actor("Root").build();
    let child = engine.spawn_actor("Child").child_of(root).unwrap().build();
    let grandchild = engine.spawn_actor("Grandchild").child_of(child).unwrap().build();

    assert_eq!(child.parent(&engine), Some(root));
    assert_eq!(root.children(&engine), vec![child]);
    assert!(root.despawn(&mut engine));
    assert!(!root.is_alive(&engine));
    assert!(!child.is_alive(&engine));
    assert!(!grandchild.is_alive(&engine));
}

#[test]
fn reparenting_and_parent_component_operations_keep_caches_consistent() {
    let mut engine = Engine::new();
    let first_parent = engine.spawn_actor("First").build();
    let second_parent = engine.spawn_actor("Second").build();
    let child = engine.spawn_actor("Child").child_of(first_parent).unwrap().build();

    child.insert(&mut engine, Parent(second_parent.entity())).unwrap();
    assert!(first_parent.children(&engine).is_empty());
    assert_eq!(second_parent.children(&engine), vec![child]);

    let removed = child.remove::<Parent>(&mut engine);
    assert_eq!(removed, Some(Parent(second_parent.entity())));
    assert_eq!(child.parent(&engine), None);
    assert!(second_parent.children(&engine).is_empty());
}

#[test]
fn despawn_only_keeps_children_alive_and_orphans_them() {
    let mut engine = Engine::new();
    let root = engine.spawn_actor("Root").build();
    let child = engine.spawn_actor("Child").child_of(root).unwrap().build();

    assert!(root.despawn_only(&mut engine));
    assert!(!root.is_alive(&engine));
    assert!(child.is_alive(&engine));
    assert_eq!(child.parent(&engine), None);
}

#[test]
fn managed_hierarchy_components_cannot_be_mutated_through_generic_access() {
    let mut engine = Engine::new();
    let root = engine.spawn_actor("Root").build();
    let child = engine.spawn_actor("Child").child_of(root).unwrap().build();

    assert!(child.get_component_mut::<Parent>(&mut engine).is_none());
    assert_eq!(child.parent(&engine), Some(root));
}

#[test]
fn hierarchy_cycles_are_rejected() {
    let mut engine = Engine::new();
    let root = engine.spawn_actor("Root").build();
    let child = engine.spawn_actor("Child").child_of(root).unwrap().build();

    assert!(matches!(
        root.set_parent(&mut engine, child),
        Err(ActorError::HierarchyCycle { .. })
    ));
}

#[test]
fn actor_first_access_and_queries_hide_world_from_game_code() {
    let mut engine = Engine::new();
    let player = engine.spawn_actor("Player").with(Health(100)).build();
    let enemy = engine.spawn_actor("Enemy").with(Health(75)).build();

    player.get_component_mut::<Health>(&mut engine).unwrap().0 -= 10;

    let mut actors = engine
        .actors_with::<Health>()
        .into_iter()
        .map(|(actor, health)| (actor, *health))
        .collect::<Vec<_>>();
    actors.sort_by_key(|(actor, _)| actor.entity());

    assert_eq!(player.get_component::<Health>(&engine).copied(), Some(Health(90)));
    assert_eq!(actors, vec![(player, Health(90)), (enemy, Health(75))]);
}
