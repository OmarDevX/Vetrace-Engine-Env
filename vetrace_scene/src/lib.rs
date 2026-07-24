//! Versioned scene/prefab JSON for Vetrace.
//!
//! `vetrace_scene` owns authored scene documents. A full map and a prefab are
//! the same thing here: a serializable scene graph that can be instantiated into
//! normal ECS entities. Editor/tool crates edit this data; games consume it.

pub mod ids;
pub mod component;
pub mod document;
pub mod assets;
pub mod material;
pub mod node;
pub mod transform;
pub mod io;
pub mod instance;
pub mod prefab;
mod export;
mod legacy;

pub use component::SceneComponent;
pub use assets::{load_scene_material_textures, load_scene_render_textures, SceneTextureLoadReport};
pub use document::{SceneDocument, SceneResources};
pub use ids::{component_type, PREFAB_VERSION, SCENE_VERSION};
pub use io::{load_scene_file, migrate_scene_file, parse_scene_text, save_scene_file};
pub use instance::SceneInstance;
pub use prefab::{PrefabBuilder, SceneEngineExt};
pub use material::SceneMaterial;
pub use node::{SceneAudioSource, SceneNode, ScenePrefabInstance, ScenePrimitive, SceneSpawnPoint, SceneWorldLabel};
pub use transform::SceneTransform;

use std::path::Path;

use anyhow::Result;
use vetrace_physics::{PhysicsBodyDef, PhysicsColliderDef};

// Compatibility aliases for the first map-builder implementation.
pub type PrefabDocument = SceneDocument;
pub type PrefabObject = SceneNode;
pub type PrefabTransform = SceneTransform;
pub type PrefabMaterial = SceneMaterial;
pub type PrefabCollider = PhysicsColliderDef;
pub type PrefabColliderShape = vetrace_physics::PhysicsColliderShapeDef;
pub type PrefabRigidBody = PhysicsBodyDef;

pub fn load_prefab_file(path: impl AsRef<Path>) -> Result<PrefabDocument> { load_scene_file(path) }
pub fn save_prefab_file(path: impl AsRef<Path>, document: &PrefabDocument) -> Result<()> { save_scene_file(path, document) }

#[cfg(test)]
mod tests {
    use vetrace_core::{Engine, Timer};

    use super::SceneDocument;

    #[test]
    fn registered_serializable_components_round_trip_through_scene_documents() {
        let mut source = Engine::new();
        source
            .spawn_actor("Timed Actor")
            .with(Timer::repeating(2.5))
            .build();

        let document = SceneDocument::from_engine(&source, "Registry Round Trip");
        assert_eq!(document.object_count(), 1);
        assert!(document.roots[0]
            .components
            .iter()
            .any(|component| component.type_id == "vetrace.core.timer"));

        let mut destination = Engine::new();
        let instance = document.instantiate(&mut destination).unwrap();
        let actor = instance.roots[0];
        let timer = actor.get_component::<Timer>(&destination).unwrap();
        assert_eq!(timer.duration, 2.5);
        assert!(timer.repeating);
    }

    #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize, vetrace_core::VetraceComponent)]
    #[vetrace_component(id = "test.authored_settings", display_name = "Authored Settings", category = "Tests")]
    struct AuthoredSettings {
        value: f32,
        #[vetrace(runtime_only)]
        runtime_cache: f32,
    }

    #[test]
    fn reflected_runtime_only_fields_do_not_leak_into_scene_files() {
        let mut source = Engine::new();
        source
            .get_resource_mut::<vetrace_core::ComponentManager>()
            .unwrap()
            .register_reflected::<AuthoredSettings>();
        source
            .spawn_actor("Reflected")
            .with(AuthoredSettings { value: 4.0, runtime_cache: 99.0 })
            .build();

        let document = SceneDocument::from_engine(&source, "Reflected Scene");
        let component = document.roots[0]
            .components
            .iter()
            .find(|component| component.type_id == "test.authored_settings")
            .unwrap();
        assert_eq!(component.data["value"], serde_json::json!(4.0));
        assert!(component.data.get("runtime_cache").is_none());

        let mut destination = Engine::new();
        destination
            .get_resource_mut::<vetrace_core::ComponentManager>()
            .unwrap()
            .register_reflected::<AuthoredSettings>();
        let instance = document.instantiate(&mut destination).unwrap();
        let settings = instance.roots[0].get_component::<AuthoredSettings>(&destination).unwrap();
        assert_eq!(settings.value, 4.0);
        assert_eq!(settings.runtime_cache, 0.0);
    }

}
