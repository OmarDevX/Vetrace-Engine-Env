use std::error::Error;
use std::net::SocketAddr;

use glam::{Mat3, Quat, Vec2, Vec3};
use vetrace_core::App;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::{Actor, Entity, GlobalTransform, InputState, Parent, Transform};
use vetrace_physics::{raycast_colliders, AngularVelocity, CharacterBody3D, CharacterController3D, CharacterControllerState, Collider, ColliderShape, KinematicBody, RigidBody3D, StaticBody, Velocity};
use vetrace_net::{
    network_actor, ClientGameEvent, ClientTimeout, CompatibilityManifest, GameClientDriver,
    GameServerDriver, ReplicatedComponentAdapter, ReplicationAuthority, RpcTarget,
    ServerGameEvent, TypedUdpChannel,
};
use vetrace_pathfinding::{NavigationGrid, PathfindingSettings, PathfindingWorld};
use vetrace_render::{
    bake_and_save_baked_lighting, cycle_baked_lighting_debug_mode, load_baked_lighting,
    set_baked_lighting_runtime_mode, unload_baked_lighting, Atmosphere,
    BakedLightingBakeConfig, BakedLightingRuntimeMode, BakedLightmapReceiver, BakedLightProbeDebugMarker, BakedLightProbeReceiver, Camera, CustomPostProcessPass, CustomPostProcessStack, CustomShaderCullMode,
    CustomShaderDepthCompare, CustomShaderMaterial, CustomShaderRenderBucket, DirectionalLight,
    EmissiveLightEmitter, Material, PostProcessInput, PresentModePreference, PrimitiveShape, RenderSettings, Renderable, ScreenSpaceRect,
    ShadowMode, Shape, VolumetricFog,
};
use vetrace_ui::{Anchor, TextAlign, UILabel, UIWorldSpace};
#[cfg(feature = "audio")]
use vetrace_audio::{AudioListener, AudioSource};

use crate::components::*;
use crate::net::{
    v3, MapManifest, NetInput, PlayerSnapshot, ServerSnapshot, ShooterAdminCommand,
    ShooterHello, ShooterMessage, ShooterModSettings, ShooterPacket, ShooterRpc,
    ShooterWelcome, ShotSnapshot, SHOOTER_PROTOCOL_VERSION,
};
use crate::replication::{TransformInterpolator, TransformReplicator, TransformSnapshot};

const SERVER_SNAPSHOT_RATE_HZ: f32 = 30.0;
const REMOTE_INTERPOLATION_SECONDS: f32 = 0.075;
const LOCAL_RECONCILE_HARD_SNAP_DISTANCE_SQ: f32 = 9.0;
const LOCAL_RECONCILE_SOFT_DISTANCE_SQ: f32 = 0.04;
const LOCAL_RECONCILE_SOFT_ALPHA: f32 = 0.35;

pub(crate) type ShooterServerNet = GameServerDriver<
    NetInput, ShooterRpc, PlayerSnapshot, ShotSnapshot, ShooterMessage,
    ShooterHello, ShooterWelcome, ShooterClientData,
>;
pub(crate) type ShooterClientNet = GameClientDriver<
    NetInput, ShooterRpc, PlayerSnapshot, ShotSnapshot, ShooterMessage,
    ShooterHello, ShooterWelcome, PredictedInput,
>;

fn shooter_compatibility(engine: &Engine) -> CompatibilityManifest {
    CompatibilityManifest::new(SHOOTER_PROTOCOL_VERSION)
        .with_gameplay_hash("weapons", weapon_gameplay_fingerprint(engine))
}

const FREE_FLIGHT_MIN_SPEED: f32 = 0.05;
const FREE_FLIGHT_MAX_SPEED: f32 = 100_000.0;
const FREE_FLIGHT_SPEED_STEP: f32 = 1.25;
const FREE_FLIGHT_SHIFT_MULTIPLIER: f32 = 4.0;
const FREE_FLIGHT_CONTROL_MULTIPLIER: f32 = 0.25;

const PLAYER_GRADIENT_SHADER_ID: &str = "simple_shooter/player_gradient";
const PLAYER_GRADIENT_SHADER_PATH: &str = "assets/player_gradient.wgsl";
const PLAYER_GRADIENT_SHADER_SOURCE: &str = include_str!("../assets/player_gradient.wgsl");
const PLAYER_OUTLINE_SHADER_ID: &str = "simple_shooter/player_outline_shell";
const PLAYER_OUTLINE_SHADER_PATH: &str = "assets/player_outline.wgsl";
const PLAYER_OUTLINE_SHADER_SOURCE: &str = include_str!("../assets/player_outline.wgsl");


// Explicit modules keep subsystem dependencies compiler-visible. The parent
// imports only each subsystem's declared `pub(super)` surface.
mod audio;
mod bot_ai;
mod camera_input;
mod free_flight;
mod gameplay;
mod health_feedback;
mod join_menu;
mod killcam;
mod lobby;
mod main_menu;
mod map_transfer;
mod maps;
mod modding;
mod pause_menu;
mod pipeline;
mod player_visuals;
pub(crate) mod runtime;
mod server_browser;
mod setup;
mod simulation;
mod weapons;

use audio::*;
use bot_ai::*;
use camera_input::*;
use free_flight::*;
use gameplay::*;
use health_feedback::*;
use join_menu::*;
use killcam::*;
use lobby::*;
use main_menu::*;
use map_transfer::*;
use maps::*;
use modding::*;
use pause_menu::*;
use pipeline::*;
use player_visuals::*;
pub(crate) use runtime::*;
use server_browser::*;
use setup::*;
use simulation::*;
use weapons::*;
