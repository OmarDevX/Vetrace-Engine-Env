use serde::{de::DeserializeOwned, Deserialize, Serialize};
use glam::Vec3;
use vetrace_core::{Actor, ActorId, Engine, Entity, Name, Transform};
use vetrace_physics::{apply_physics_defs, body_def_from_entity, collider_def_from_entity, PhysicsBodyDef, PhysicsColliderDef};
use vetrace_primitives::{spawn_primitive_actor, PrimitiveColliderOptions, PrimitiveKind, PrimitiveSpawnOptions};
use vetrace_render::{Material, Shape};

use crate::component::SceneComponent;
use crate::SceneInstance;
use crate::ids::{component_type, MATERIAL_ALIASES, PHYSICS_BODY_ALIASES, PHYSICS_COLLIDER_ALIASES, PRIMITIVE_ALIASES, TAGS_ALIASES};
use crate::material::SceneMaterial;
use crate::transform::SceneTransform;
use crate::export::{export_tags, has_exportable_children, is_helper, stable_or_random_id};
use crate::legacy::LegacyPrefabObject;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: String,
    pub name: String,
    pub transform: SceneTransform,
    #[serde(default)]
    pub components: Vec<SceneComponent>,
    #[serde(default)]
    pub children: Vec<SceneNode>,
}

impl SceneNode {
    pub fn from_entity(engine: &Engine, entity: Entity) -> Option<Self> {
        if is_helper(engine, entity) { return None; }
        let transform = engine.raw_world().get::<Transform>(entity).cloned().unwrap_or_default();
        let name = engine.raw_world().get::<Name>(entity)
            .map(|name| name.0.clone())
            .unwrap_or_else(|| format!("Entity_{}", entity.0));

        let mut components = Vec::new();
        if let Some(shape) = engine.raw_world().get::<Shape>(entity) {
            components.push(SceneComponent::new(component_type::PRIMITIVE, ScenePrimitive {
                kind: PrimitiveKind::from(shape.primitive),
                size: shape.size.to_array(),
                visible: engine.raw_world().get::<vetrace_render::Renderable>(entity).map(|r| r.visible).unwrap_or(true),
            }));
        }
        if let Some(material) = engine.raw_world().get::<Material>(entity) {
            components.push(SceneComponent::new(component_type::MATERIAL, SceneMaterial::from_material(material)));
        }
        if let Some(body) = body_def_from_entity(engine, entity) {
            components.push(SceneComponent::new(component_type::PHYSICS_BODY, body));
        }
        if let Some(collider) = collider_def_from_entity(engine, entity) {
            components.push(SceneComponent::new(component_type::PHYSICS_COLLIDER, collider));
        }
        let tags = export_tags(engine, entity);
        if !tags.is_empty() {
            components.push(SceneComponent::new(component_type::TAGS, tags));
        }

        // Persist any plugin component that opted into the generic registry.
        // Core identity/transform/name/hierarchy data is already represented by
        // the scene node itself and must not be duplicated in the component list.
        if let Some(actor) = engine.actor(entity) {
            for (stable_id, data) in engine.serialize_registered_components(actor) {
                if !is_reserved_registry_component(&stable_id) && !is_builtin_scene_component(&stable_id) {
                    components.push(SceneComponent::raw(stable_id, data));
                }
            }
        }

        if components.is_empty() && !has_exportable_children(engine, entity) {
            return None;
        }

        Some(Self {
            id: stable_or_random_id(engine, entity),
            name,
            transform: SceneTransform::from_transform(&transform),
            components,
            children: Vec::new(),
        })
    }

    pub fn spawn_recursive_actor(
        &self,
        engine: &mut Engine,
        parent: Option<Actor>,
        instance: &mut SceneInstance,
    ) -> Actor {
        let actor = self.spawn_one_actor(engine);
        if let Some(parent) = parent { let _ = actor.set_parent(engine, parent); }
        instance.record(engine, self.id.clone(), actor, parent.is_none());
        for child in &self.children {
            child.spawn_recursive_actor(engine, Some(actor), instance);
        }
        actor
    }

    pub fn instantiate(&self, engine: &mut Engine) -> SceneInstance {
        let mut instance = SceneInstance::default();
        self.spawn_recursive_actor(engine, None, &mut instance);
        vetrace_core::propagate_global_transforms(engine);
        instance
    }

    #[deprecated(note = "use SceneNode::instantiate or spawn_recursive_actor")]
    pub fn spawn_recursive(&self, engine: &mut Engine, parent: Option<Entity>, spawned: &mut Vec<Entity>) -> Entity {
        let mut instance = SceneInstance::default();
        let parent = parent.and_then(|entity| engine.actor(entity));
        let root = self.spawn_recursive_actor(engine, parent, &mut instance);
        spawned.extend(instance.actors.into_iter().map(Actor::entity));
        root.entity()
    }

    #[deprecated(note = "use SceneNode::instantiate")]
    pub fn spawn(&self, engine: &mut Engine) -> Entity { self.instantiate(engine).roots[0].entity() }

    fn spawn_one_actor(&self, engine: &mut Engine) -> Actor {
        let primitive = self.primitive();
        let transform = self.transform.to_transform();
        let authored_material = self.material();
        let physics_body = self.physics_body();
        let collider = self.collider();
        let tags = self.tags().unwrap_or_default();
        let stable_id = uuid::Uuid::parse_str(&self.id).map(ActorId::from_uuid).unwrap_or_default();

        let actor = if let Some(primitive) = primitive {
            let material = authored_material.clone().unwrap_or_default();
            let mut authored_tags = tags;
            if !authored_tags.iter().any(|tag| tag == vetrace_primitives::tags::PREFAB_OBJECT) {
                authored_tags.push(vetrace_primitives::tags::PREFAB_OBJECT.to_string());
            }
            let actor = spawn_primitive_actor(engine, PrimitiveSpawnOptions {
                name: self.name.clone(),
                primitive: primitive.kind,
                translation: transform.translation,
                rotation: transform.rotation,
                scale: transform.scale,
                size: Vec3::from_array(primitive.size).max(Vec3::splat(0.001)),
                color: material.base_color_vec3(),
                visible: primitive.visible,
                tags: authored_tags,
                collider: PrimitiveColliderOptions::disabled(),
            });
            let _ = actor.insert(engine, material.to_material());
            apply_physics_defs(engine, actor.entity(), physics_body.as_ref(), collider.as_ref());
            actor
        } else {
            let actor = engine
                .spawn_actor(self.name.clone())
                .with(transform)
                .source("vetrace_scene")
                .build();
            if let Some(material) = authored_material {
                let _ = actor.insert(engine, material.to_material());
            }
            for tag in tags { let _ = actor.add_tag(engine, tag); }
            apply_physics_defs(engine, actor.entity(), physics_body.as_ref(), collider.as_ref());
            actor
        };
        let _ = actor.set_id(engine, stable_id);

        // Unknown component IDs remain in the document for forward
        // compatibility. Registered and deserializable plugin components are
        // restored without adding a dependency from vetrace_scene to the plugin.
        for component in &self.components {
            if is_builtin_scene_component(&component.type_id) || is_reserved_registry_component(&component.type_id) {
                continue;
            }
            let _ = engine.apply_registered_component(actor, &component.type_id, component.data.clone());
        }
        actor
    }

    pub fn component<T: DeserializeOwned>(&self, aliases: &[&str]) -> Option<T> {
        self.components
            .iter()
            .find(|component| component.matches_any(aliases))
            .and_then(SceneComponent::decode::<T>)
    }

    pub fn components<T: DeserializeOwned>(&self, aliases: &[&str]) -> Vec<T> {
        self.components
            .iter()
            .filter(|component| component.matches_any(aliases))
            .filter_map(SceneComponent::decode::<T>)
            .collect()
    }

    pub fn primitive(&self) -> Option<ScenePrimitive> { self.component(PRIMITIVE_ALIASES) }
    pub fn material(&self) -> Option<SceneMaterial> { self.component(MATERIAL_ALIASES) }
    pub fn physics_body(&self) -> Option<PhysicsBodyDef> { self.component(PHYSICS_BODY_ALIASES) }
    pub fn collider(&self) -> Option<PhysicsColliderDef> { self.component(PHYSICS_COLLIDER_ALIASES) }
    pub fn tags(&self) -> Option<Vec<String>> { self.component(TAGS_ALIASES) }

    pub(crate) fn from_legacy_object(object: LegacyPrefabObject) -> Self {
        let mut components = vec![
            SceneComponent::new(component_type::PRIMITIVE, ScenePrimitive { kind: object.primitive, size: object.size, visible: true }),
            SceneComponent::new(component_type::MATERIAL, object.material),
        ];
        if let Some(collider) = object.collider {
            components.push(SceneComponent::new(component_type::PHYSICS_BODY, PhysicsBodyDef::default()));
            components.push(SceneComponent::new(component_type::PHYSICS_COLLIDER, collider));
        }
        if !object.tags.is_empty() {
            components.push(SceneComponent::new(component_type::TAGS, object.tags));
        }
        Self { id: object.id, name: object.name, transform: object.transform, components, children: Vec::new() }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScenePrimitive {
    pub kind: PrimitiveKind,
    pub size: [f32; 3],
    #[serde(default = "default_true")]
    pub visible: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneSpawnPoint {
    #[serde(default)]
    pub team: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneWorldLabel {
    pub text: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SceneAudioSource {
    pub path: String,
    #[serde(default)]
    pub spatial: bool,
    #[serde(default)]
    pub looping: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScenePrefabInstance {
    pub path: String,
}

fn is_builtin_scene_component(type_id: &str) -> bool {
    PRIMITIVE_ALIASES.contains(&type_id)
        || MATERIAL_ALIASES.contains(&type_id)
        || PHYSICS_BODY_ALIASES.contains(&type_id)
        || PHYSICS_COLLIDER_ALIASES.contains(&type_id)
        || TAGS_ALIASES.contains(&type_id)
}

fn is_reserved_registry_component(type_id: &str) -> bool {
    matches!(
        type_id,
        "vetrace.core.actor_id"
            | "vetrace.core.transform"
            | "vetrace.core.global_transform"
            | "vetrace.core.name"
            | "vetrace.core.parent"
            | "vetrace.core.children_compat"
            | "vetrace.core.metadata"
            | "vetrace.core.transform_dirty"
    )
}

fn default_true() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_only_nodes_restore_their_authored_material() {
        let expected = SceneMaterial {
            base_color: [0.12, 0.34, 0.56],
            roughness: 0.27,
            metallic: 0.18,
            alpha: 0.73,
            ..SceneMaterial::default()
        };
        let node = SceneNode {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Material Only".to_owned(),
            transform: SceneTransform::default(),
            components: vec![SceneComponent::new(component_type::MATERIAL, expected.clone())],
            children: Vec::new(),
        };

        let mut engine = Engine::new();
        let instance = node.instantiate(&mut engine);
        let actor = instance.roots[0];
        let material = actor.get_component::<Material>(&engine).expect("material-only scene nodes must restore Material");

        assert_eq!(material.base_color.to_array(), expected.base_color);
        assert_eq!(material.roughness, expected.roughness);
        assert_eq!(material.metallic, expected.metallic);
        assert_eq!(material.alpha, expected.alpha);
    }
}
