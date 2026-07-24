use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use vetrace_core::Entity;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LerpState {
    Idle,
    Playing,
    Finished,
}

impl Default for LerpState {
    fn default() -> Self { Self::Idle }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopMode {
    Once,
    Repeat,
    PingPong,
}

impl Default for LoopMode {
    fn default() -> Self { Self::Once }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Default for Easing {
    fn default() -> Self { Self::Linear }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LerpData {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub easing: Easing,
    pub loop_mode: LoopMode,
    pub state: LerpState,
}

impl Default for LerpData {
    fn default() -> Self {
        Self {
            from: 0.0,
            to: 1.0,
            duration: 1.0,
            elapsed: 0.0,
            easing: Easing::Linear,
            loop_mode: LoopMode::Once,
            state: LerpState::Idle,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Lerp {
    pub channels: Vec<LerpData>,
}

/// Legacy/simple clock component kept for existing users.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    pub name: String,
    pub length_seconds: f32,
    pub time_seconds: f32,
    pub speed: f32,
    pub loop_mode: LoopMode,
    pub playing: bool,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            name: String::new(),
            length_seconds: 1.0,
            time_seconds: 0.0,
            speed: 1.0,
            loop_mode: LoopMode::Repeat,
            playing: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationInterpolation {
    Step,
    Linear,
    CubicSpline,
}

impl Default for AnimationInterpolation {
    fn default() -> Self { Self::Linear }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimationTargetProperty {
    Translation,
    Rotation,
    Scale,
    MorphWeights,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnimationOutputValues {
    Vec3(Vec<Vec3>),
    Quat(Vec<Quat>),
    /// Flattened keyframes for morph target weights. `width` is the number of
    /// weights per keyframe.
    Weights { width: usize, values: Vec<f32> },
}

impl AnimationOutputValues {
    pub fn key_count(&self) -> usize {
        match self {
            Self::Vec3(values) => values.len(),
            Self::Quat(values) => values.len(),
            Self::Weights { width, values } => {
                if *width == 0 { 0 } else { values.len() / *width }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationSampler {
    pub inputs: Vec<f32>,
    pub interpolation: AnimationInterpolation,
    pub outputs: AnimationOutputValues,
}

impl AnimationSampler {
    pub fn duration(&self) -> f32 {
        self.inputs.iter().copied().fold(0.0_f32, f32::max)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationChannel {
    pub target: Entity,
    pub property: AnimationTargetProperty,
    pub sampler: AnimationSampler,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationPlayer {
    pub clips: Vec<AnimationClip>,
    pub active_clip: usize,
    pub time_seconds: f32,
    pub speed: f32,
    pub loop_mode: LoopMode,
    pub playing: bool,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            clips: Vec::new(),
            active_clip: 0,
            time_seconds: 0.0,
            speed: 1.0,
            loop_mode: LoopMode::Repeat,
            playing: true,
        }
    }
}

impl AnimationPlayer {
    pub fn with_clips(clips: Vec<AnimationClip>) -> Self {
        Self { clips, ..Self::default() }
    }

    pub fn active_clip(&self) -> Option<&AnimationClip> {
        self.clips.get(self.active_clip)
    }

    pub fn play(&mut self, clip_index: usize) {
        if clip_index < self.clips.len() {
            self.active_clip = clip_index;
            self.time_seconds = 0.0;
            self.playing = true;
        }
    }

    pub fn play_named(&mut self, name: &str) -> bool {
        let Some(index) = self.clips.iter().position(|clip| clip.name == name) else {
            return false;
        };
        self.play(index);
        true
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MorphTargets {
    pub names: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MorphWeights {
    pub weights: Vec<f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Skin {
    pub joints: Vec<Entity>,
    pub inverse_bind_matrices: Vec<Mat4>,
    pub skeleton_root: Option<Entity>,
}
