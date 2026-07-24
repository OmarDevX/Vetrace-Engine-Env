//! Compatibility/distribution crate for Vetrace.
//!
//! New framework users should depend on `vetrace_core` directly and add only
//! the plugin crates they need. This crate re-exports core and optional official
//! plugins for users who still want a single package.

pub use vetrace_core::*;

#[cfg(feature = "asset")]
pub use vetrace_asset as asset;
#[cfg(feature = "build")]
pub use vetrace_build as build;

#[cfg(feature = "render")]
pub use vetrace_render as render;
#[cfg(feature = "physics")]
pub use vetrace_physics as physics;
#[cfg(feature = "lua")]
pub use vetrace_scripting_lua as scripting_lua;
#[cfg(feature = "script_tools")]
pub use vetrace_script_editor as script_editor;
#[cfg(feature = "script_tools")]
pub use vetrace_lua_tools as lua_tools;
#[cfg(feature = "net")]
pub use vetrace_net as net;
#[cfg(feature = "ui")]
pub use vetrace_ui as ui;
#[cfg(feature = "animation")]
pub use vetrace_animation as animation;
#[cfg(feature = "audio")]
pub use vetrace_audio as audio;
#[cfg(feature = "editor")]
pub use vetrace_editor as editor;
#[cfg(feature = "profiler")]
pub use vetrace_profiler as profiler;
#[cfg(feature = "primitives")]
pub use vetrace_primitives as primitives;
#[cfg(feature = "scene")]
pub use vetrace_scene as scene;
#[cfg(feature = "prefab")]
pub use vetrace_scene as prefab;

/// Prelude for framework-style Rust games.
pub mod prelude {
    #[cfg(feature = "asset")]
    pub use vetrace_asset::{
        register_builtin_importers, AssetDatabase, AssetId, AssetImporter, AssetKind,
        AssetManager, AssetRecord, AssetStatus, AssetWatcher, ImporterRegistry,
    };
    #[cfg(feature = "build")]
    pub use vetrace_build::{
        build_project, create_vpak, inspect_vpak, mount_vpak, BuildAssetPreflight, BuildReport, BuildRequest,
        ExportConfig, ExportPreset, ExportTarget, PackageManifest,
    };
    pub use vetrace_core::{
        Actor, ActorBuilder, ActorDestroyed, ActorError, ActorId, App, AppBuilder,
        Bundle, Commands, Component, ComponentDescriptor, ComponentManager,
        ComponentSchema, DynamicValue, Engine, Entity, EventReader, EventWriter,
        Events, FieldKind, FieldPath, FieldSchema, FixedTime, GlobalTransform,
        Metadata, Name, NumericRange, ObjectRef, Parent, Plugin, Query,
        ReflectionError, Schedule, Stage, Timer, Transform, VetraceComponent, World,
    };
    #[cfg(feature = "render")]
    pub use vetrace_render::{RenderActorExt, RenderBundle, RenderPlugin};
    #[cfg(feature = "render_2d")]
    pub use vetrace_render::{
        BlendMode2D, Camera2D, CanvasItem2D, Rect2D, Render2dPlugin, Sprite2D,
        Sprite2DBundle, TextureFilter2D, Transform2DExt,
    };
    #[cfg(feature = "physics")]
    pub use vetrace_physics::{CharacterBodyBundle, PhysicsActorExt, RapierPhysicsPlugin, RigidBodyBundle};
    #[cfg(feature = "physics_2d")]
    pub use vetrace_physics::{
        overlap_box_2d, overlap_circle_2d, point_query_2d, raycast_2d, BodyType2D,
        Collider2D, ColliderShape2D, CollisionContact2D, CollisionStarted2D,
        CollisionStopped2D, Physics2dActorExt, Physics2dPlugin, Physics2dQueryFilter,
        Physics2dState, Physics2dStats, RaycastHit2D, RigidBody2D, RigidBody2dBundle,
        Velocity2D,
    };
    #[cfg(feature = "lua")]
    pub use vetrace_scripting_lua::LuaScriptingPlugin;
    #[cfg(feature = "script_tools")]
    pub use vetrace_lua_tools::LuaLanguageService;
    #[cfg(feature = "script_tools")]
    pub use vetrace_script_editor::{
        CodeAction, CompletionItem, DiagnosticSeverity, HighlightKind, LanguageContext,
        LanguageRegistry, ScriptDiagnostic, ScriptDocument, ScriptLanguageService,
        ScriptWorkspace, TextEdit, TextPosition, TextRange,
    };
    #[cfg(feature = "net")]
    pub use vetrace_net::NetPlugin;
    #[cfg(feature = "ui")]
    pub use vetrace_ui::UiPlugin;
    #[cfg(feature = "animation")]
    pub use vetrace_animation::AnimationPlugin;
    #[cfg(feature = "audio")]
    pub use vetrace_audio::AudioPlugin;
    #[cfg(feature = "editor")]
    pub use vetrace_editor::{editor, EditorConfig, EditorGizmoMode, EditorOnly, EditorPlugin, EditorState, EditorTool};
    #[cfg(feature = "profiler")]
    pub use vetrace_profiler::{profile_scope, ProfilerConfig, ProfilerPlugin, ProfilerReport, ProfilerUiMode, ProfilerUiSettings, ScopeTimer};
    #[cfg(feature = "primitives")]
    pub use vetrace_primitives::{spawn_primitive_actor, PrimitiveColliderOptions, PrimitiveKind, PrimitiveSpawnOptions};
    #[cfg(all(feature = "primitives", feature = "render_2d"))]
    pub use vetrace_primitives::{spawn_sprite_2d_actor, Sprite2DSpawnOptions};
    #[cfg(feature = "scene")]
    pub use vetrace_scene::{
        load_scene_file, save_scene_file, SceneComponent, SceneDocument, SceneEngineExt,
        SceneInstance, SceneNode,
    };
    #[cfg(feature = "prefab")]
    pub use vetrace_scene::{load_prefab_file, save_prefab_file, PrefabDocument, PrefabObject};
}
