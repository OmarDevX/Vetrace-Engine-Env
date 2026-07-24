use crate::{ActorDestroyed, ActorId, Engine, Events, Stage, Transform};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Health(i32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Enemy;
#[derive(Clone, Copy, Debug, PartialEq)]
struct Speed(f32);

#[test]
fn actor_ids_are_unique_and_survive_runtime_handle_changes() {
    let mut engine = Engine::new();
    let first = engine.spawn_actor("First").build();
    let first_id = first.id(&engine).unwrap();
    first.despawn(&mut engine);
    let second = engine.spawn_actor("Second").build();
    assert_ne!(first.entity(), second.entity());
    assert_ne!(first_id, second.id(&engine).unwrap());
    assert_eq!(engine.find_actor_by_id(second.id(&engine).unwrap()), Some(second));
}

#[test]
fn tuple_queries_filters_and_mutation_work_without_world_access() {
    let mut engine = Engine::new();
    let enemy = engine.spawn_actor("Enemy").with(Health(10)).with(Speed(2.0)).with(Enemy).build();
    engine.spawn_actor("Friend").with(Health(20)).with(Speed(4.0)).build();

    let rows = engine.query::<(&Health, &Speed)>().with::<Enemy>().collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, enemy);

    engine.query_mut_with::<Health, Speed>().with::<Enemy>().for_each(|_, health, speed| {
        health.0 += speed.0 as i32;
    });
    assert_eq!(enemy.get_component::<Health>(&engine), Some(&Health(12)));
}

#[test]
fn deferred_commands_apply_structural_changes_safely() {
    let mut engine = Engine::new();
    let actor = engine.spawn_actor("Temporary").with(Health(1)).build();
    engine.defer(|commands| {
        commands.insert(actor, Enemy);
        commands.remove::<Health>(actor);
    });
    assert!(actor.has::<Health>(&engine));
    engine.flush_commands();
    assert!(!actor.has::<Health>(&engine));
    assert!(actor.has::<Enemy>(&engine));
}

#[test]
fn typed_events_can_be_written_read_and_drained() {
    let mut engine = Engine::new();
    engine.send_event(Health(7));
    assert_eq!(engine.event_reader::<Health>().iter().copied().collect::<Vec<_>>(), vec![Health(7)]);
    assert_eq!(engine.drain_events::<Health>(), vec![Health(7)]);
    assert!(engine.event_reader::<Health>().is_empty());
}

#[test]
fn actor_destruction_emits_a_typed_event() {
    let mut engine = Engine::new();
    let actor = engine.spawn_actor("Disposable").build();
    let id = actor.id(&engine);
    actor.despawn(&mut engine);
    let events = engine.drain_events::<ActorDestroyed>();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor, actor);
    assert_eq!(events[0].id, id);
}

#[test]
fn component_registry_uses_stable_ids() {
    let engine = Engine::new();
    let registry = engine.get_resource::<crate::ComponentManager>().unwrap();
    let descriptor = registry.descriptor("vetrace.core.transform").unwrap();
    assert_eq!(descriptor.display_name, "Transform");
    assert!(descriptor.serialize.is_some());
    let actor_id = registry.descriptor("vetrace.core.actor_id").unwrap();
    assert!(actor_id.clone_component.is_none());
    assert!(actor_id.deserialize.is_none());
}

#[test]
fn fixed_schedule_runs_named_stages() {
    let mut engine = Engine::new();
    engine.insert_resource(Vec::<Stage>::new());
    engine.add_system(Stage::Update, "record_update", |engine, _| {
        engine.get_resource_mut::<Vec<Stage>>().unwrap().push(Stage::Update);
    });
    engine.run_stage(Stage::Update, 1.0 / 60.0);
    assert_eq!(engine.get_resource::<Vec<Stage>>().unwrap(), &vec![Stage::Update]);
}

#[test]
fn failed_builder_parenting_rolls_back() {
    let mut engine = Engine::new();
    let dead_parent = engine.spawn_actor("Dead").build();
    dead_parent.despawn(&mut engine);
    let count_before = engine.actors().len();
    let failed = engine.spawn_actor("Child").child_of(dead_parent).is_err();
    assert!(failed);
    assert_eq!(engine.actors().len(), count_before);
}

#[test]
fn actor_id_is_serializable_but_not_copied_by_registry_clone() {
    let mut engine = Engine::new();
    let source = engine.spawn_actor("Source").with(Transform::default()).build();
    let target = engine.spawn_actor("Target").build();
    let original_target_id = target.id(&engine).unwrap();
    let failures = engine.clone_registered_components(source, target);
    assert!(failures.is_empty());
    assert_eq!(target.id(&engine), Some(original_target_id));
}

#[test]
fn events_resource_remains_available_as_a_normal_resource() {
    let mut engine = Engine::new();
    engine.send_event(ActorId::new());
    assert!(engine.contains_resource::<Events<ActorId>>());
}

#[test]
fn stale_actor_handles_cannot_reach_reused_slots() {
    let mut engine = Engine::new();
    let first = engine.spawn_actor("First").with(Health(1)).build();
    let slot = first.entity().index();
    first.despawn(&mut engine);
    let replacement = engine.spawn_actor("Replacement").with(Health(2)).build();

    assert_eq!(replacement.entity().index(), slot);
    assert_ne!(replacement.entity().generation(), first.entity().generation());
    assert!(!first.is_alive(&engine));
    assert_eq!(first.get_component::<Health>(&engine), None);
    assert_eq!(replacement.get_component::<Health>(&engine), Some(&Health(2)));
}

#[test]
fn duplicate_actor_ids_are_rejected_and_identity_is_managed() {
    let mut engine = Engine::new();
    let first = engine.spawn_actor("First").build();
    let second = engine.spawn_actor("Second").build();
    let first_id = first.id(&engine).unwrap();

    assert!(matches!(
        second.set_id(&mut engine, first_id),
        Err(crate::ActorError::DuplicateActorId(id)) if id == first_id
    ));
    assert!(matches!(
        second.insert(&mut engine, ActorId::new()),
        Err(crate::ActorError::ManagedComponent(_))
    ));
    assert!(first.get_component_mut::<ActorId>(&mut engine).is_none());
    assert!(first.remove::<ActorId>(&mut engine).is_none());
    assert_eq!(first.id(&engine), Some(first_id));
}

#[test]
fn raw_transform_changes_are_detected_and_propagated_to_descendants() {
    let mut engine = Engine::new();
    let root = engine
        .spawn_actor("Root")
        .with(Transform { translation: glam::Vec3::new(1.0, 0.0, 0.0), ..Transform::default() })
        .build();
    let child = engine
        .spawn_actor("Child")
        .with(Transform { translation: glam::Vec3::new(2.0, 0.0, 0.0), ..Transform::default() })
        .child_of(root)
        .unwrap()
        .build();

    engine.raw_world_mut().get_mut::<Transform>(root.entity()).unwrap().translation.x = 5.0;
    crate::propagate_global_transforms(&mut engine);

    assert_eq!(
        child.global_transform(&engine).map(|transform| transform.translation),
        Some(glam::Vec3::new(7.0, 0.0, 0.0)),
    );
}

struct FailingBundle;

impl crate::Bundle for FailingBundle {
    fn insert(self, actor: crate::Actor, engine: &mut Engine) -> Result<(), crate::ActorError> {
        actor.insert(engine, Health(99))?;
        Err(crate::ActorError::ManagedComponent("intentional test failure"))
    }
}

#[test]
fn failed_builder_rolls_back_components_and_hierarchy_index() {
    let mut engine = Engine::new();
    let parent = engine.spawn_actor("Parent").build();
    let result = engine
        .spawn_actor("Incomplete")
        .child_of(parent)
        .unwrap()
        .bundle(FailingBundle)
        .try_build();

    assert!(result.is_err());
    assert!(parent.children(&engine).is_empty());
    assert!(engine.find_actor_by_name("Incomplete").is_none());
}

#[test]
fn mutable_queries_can_queue_structural_commands() {
    let mut engine = Engine::new();
    let actor = engine.spawn_actor("Doomed").with(Health(0)).build();
    engine.query_mut::<Health>().for_each_with_commands(|actor, health, commands| {
        if health.0 <= 0 { commands.despawn(actor); }
    });
    assert!(actor.is_alive(&engine));
    engine.flush_commands();
    assert!(!actor.is_alive(&engine));
}

#[test]
fn built_in_transform_schedule_refreshes_globals() {
    let mut engine = Engine::new();
    let actor = engine.spawn_actor("Moved").build();
    actor.set_position(&mut engine, glam::Vec3::new(3.0, 4.0, 5.0)).unwrap();
    engine.run_stage(Stage::PostUpdate, 0.0);
    assert_eq!(
        actor.global_transform(&engine).map(|transform| transform.translation),
        Some(glam::Vec3::new(3.0, 4.0, 5.0)),
    );
}


#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, crate::VetraceComponent)]
#[vetrace_component(
    id = "test.health",
    display_name = "Health",
    category = "Gameplay",
    description = "Custom reflected component used to prove open registration"
)]
struct ReflectedHealth {
    #[vetrace(min = 0.0, max = 100.0, step = 1.0)]
    current: f32,
    #[vetrace(min = 1.0)]
    maximum: f32,
    #[vetrace(read_only)]
    revision: u32,
}

#[test]
fn reflected_custom_components_need_no_core_type_switch() {
    let mut engine = Engine::new();
    engine
        .get_resource_mut::<crate::ComponentManager>()
        .unwrap()
        .register_reflected::<ReflectedHealth>();

    let actor = engine.spawn_actor("Reflected").build();
    engine
        .add_registered_component(
            actor,
            "test.health",
            Some(crate::DynamicValue::from_json(serde_json::json!({
                "current": 75.0,
                "maximum": 100.0
            }))),
        )
        .unwrap();

    let current = crate::FieldPath::parse("current").unwrap();
    assert_eq!(
        engine.registered_component_field(actor, "Health", &current).unwrap(),
        crate::DynamicValue::F64(75.0)
    );
    engine
        .set_registered_component_field(
            actor,
            "test.health",
            &current,
            crate::DynamicValue::F64(25.0),
        )
        .unwrap();
    assert_eq!(actor.get_component::<ReflectedHealth>(&engine).unwrap().current, 25.0);

    let revision = crate::FieldPath::parse("revision").unwrap();
    assert!(engine
        .set_registered_component_field(
            actor,
            "test.health",
            &revision,
            crate::DynamicValue::U64(2),
        )
        .is_err());

    let schema = engine.component_schema(Some(actor), "test.health").unwrap();
    assert_eq!(schema.category, "Gameplay");
    assert_eq!(schema.fields.len(), 3);
    assert_eq!(schema.fields[0].numeric_range.as_ref().unwrap().max, Some(100.0));
}


#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, crate::VetraceComponent)]
#[vetrace_component(
    id = "test.reflection_policy",
    display_name = "Reflection Policy",
    category = "Tests"
)]
struct ReflectionPolicyComponent {
    editable: f32,
    #[vetrace(read_only)]
    revision: u32,
    #[vetrace(hidden_from_lua)]
    private_value: f32,
    #[vetrace(runtime_only)]
    runtime_cache: f32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, crate::VetraceComponent)]
#[vetrace_component(
    id = "test.non_constructible",
    display_name = "Non Constructible",
    category = "Tests",
    non_constructible
)]
struct NonConstructibleComponent {
    value: f32,
}

#[test]
fn reflection_policy_is_enforced_by_the_registry() {
    let mut engine = Engine::new();
    {
        let registry = engine.get_resource_mut::<crate::ComponentManager>().unwrap();
        registry.register_reflected::<ReflectionPolicyComponent>();
        registry.register_reflected::<NonConstructibleComponent>();
    }
    let actor = engine
        .spawn_actor("Policy")
        .with(ReflectionPolicyComponent {
            editable: 1.0,
            revision: 7,
            private_value: 9.0,
            runtime_cache: 11.0,
        })
        .build();

    let root = crate::FieldPath::root();
    let lua_value = engine.lua_component_field(actor, "test.reflection_policy", &root).unwrap();
    let object = lua_value.object().unwrap();
    assert_eq!(object.get("editable"), Some(&crate::DynamicValue::F64(1.0)));
    assert!(!object.contains_key("private_value"));

    let private = crate::FieldPath::parse("private_value").unwrap();
    assert!(engine.lua_component_field(actor, "test.reflection_policy", &private).is_err());
    let revision = crate::FieldPath::parse("revision").unwrap();
    assert!(engine
        .set_registered_component_field(actor, "test.reflection_policy", &revision, crate::DynamicValue::I64(8))
        .is_err());

    let serialized = engine.serialize_registered_components(actor);
    let policy = serialized["test.reflection_policy"].as_object().unwrap();
    assert!(!policy.contains_key("runtime_cache"));
    assert!(engine
        .add_registered_component(actor, "test.non_constructible", None)
        .is_err());
}

#[test]
fn core_authored_identity_components_are_not_generically_removable() {
    let engine = crate::Engine::new();
    let schemas = engine.registered_component_schemas();
    let transform = schemas
        .iter()
        .find(|schema| schema.stable_id == "vetrace.core.transform")
        .unwrap();
    let name = schemas
        .iter()
        .find(|schema| schema.stable_id == "vetrace.core.name")
        .unwrap();
    assert!(!transform.removable);
    assert!(!name.removable);
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    crate::VetraceEnum,
)]
#[serde(rename_all = "snake_case")]
enum EditorTestMode {
    #[default]
    IdleMode,
    RunningFast,
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    crate::VetraceComponent,
)]
#[vetrace_component(
    id = "test.editor_enum",
    display_name = "Editor Enum",
    category = "Tests"
)]
struct EditorEnumComponent {
    #[vetrace(enum_options)]
    mode: EditorTestMode,
}

#[test]
fn reflected_enum_fields_publish_serialized_dropdown_variants() {
    let mut engine = Engine::new();
    engine
        .get_resource_mut::<crate::ComponentManager>()
        .unwrap()
        .register_reflected::<EditorEnumComponent>();

    let descriptor = engine
        .get_resource::<crate::ComponentManager>()
        .unwrap()
        .descriptor("test.editor_enum")
        .unwrap();
    let schema = descriptor.schema.as_ref().unwrap();
    let mode = schema.fields.iter().find(|field| field.name == "mode").unwrap();

    assert_eq!(mode.kind, crate::FieldKind::Enum);
    assert_eq!(
        mode.enum_variants.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["idle_mode", "running_fast"],
    );
}
