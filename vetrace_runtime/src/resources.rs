use std::time::Duration;

use vetrace_project::{ProjectPath, VetraceProject};
use vetrace_scene::{SceneDocument, SceneInstance, SceneTextureLoadReport};

use crate::{RuntimeMode, RuntimeState};

#[derive(Clone, Debug)]
pub struct RuntimeProject(pub VetraceProject);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub wgpu_window: bool,
    pub software_window: bool,
    pub audio_backend: bool,
    pub gltf: bool,
    pub render_2d: bool,
    pub physics_2d: bool,
}

impl RuntimeCapabilities {
    pub const fn compiled() -> Self {
        Self {
            wgpu_window: cfg!(feature = "window"),
            software_window: cfg!(feature = "software_window"),
            audio_backend: cfg!(feature = "audio_backend"),
            gltf: cfg!(feature = "gltf"),
            render_2d: cfg!(feature = "render_2d"),
            physics_2d: cfg!(feature = "physics_2d"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeStatus {
    pub mode: RuntimeMode,
    pub state: RuntimeState,
    pub frame: u64,
    pub delta_seconds: f32,
    pub elapsed: Duration,
}

impl RuntimeStatus {
    pub fn new(mode: RuntimeMode) -> Self {
        Self {
            mode,
            state: RuntimeState::Created,
            frame: 0,
            delta_seconds: 0.0,
            elapsed: Duration::ZERO,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeDiagnostics {
    warnings: Vec<String>,
}

impl RuntimeDiagnostics {
    pub fn warnings(&self) -> &[String] { &self.warnings }
    pub fn push_warning(&mut self, warning: impl Into<String>) { self.warnings.push(warning.into()); }
}

#[derive(Clone, Debug)]
pub struct ActiveRuntimeScene {
    pub path: ProjectPath,
    pub document: SceneDocument,
    pub instance: SceneInstance,
    pub textures: SceneTextureLoadReport,
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeAutoloads {
    pub scripts: Vec<ProjectPath>,
}
