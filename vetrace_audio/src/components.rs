use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioPlayState {
    Stopped,
    Playing,
    Paused,
}

impl Default for AudioPlayState {
    fn default() -> Self { Self::Stopped }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioLoadMode {
    /// Load the whole clip into memory. Best for short repeated sounds like guns/UI.
    Static,
    /// Stream from disk. Best for long music tracks.
    Streaming,
}

impl Default for AudioLoadMode {
    fn default() -> Self { Self::Static }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioSource {
    /// File path. Game crates may use either an absolute path or a path relative
    /// to their current working directory, for example `assets/shoot.mp3`.
    pub path: String,
    /// Linear amplitude where 1.0 is normal volume and 0.0 is silent.
    pub volume: f32,
    /// Playback-rate multiplier. 1.0 is normal pitch/speed.
    pub pitch: f32,
    pub looping: bool,
    /// If true, play through a spatial track attached to this entity transform.
    /// If false, play globally through the main mixer.
    pub spatial: bool,
    /// Whether the backend should start this source immediately when it appears.
    pub play_on_spawn: bool,
    pub state: AudioPlayState,
    pub load_mode: AudioLoadMode,
    /// Despawn the ECS entity after a non-looping sound finishes.
    pub auto_despawn: bool,
    /// Gameplay-facing distance hint. The Kira backend already handles spatial
    /// attenuation/panning through spatial tracks; this remains as stable data
    /// for future backend-specific falloff customization.
    pub max_distance: f32,
}

impl Default for AudioSource {
    fn default() -> Self {
        Self {
            path: String::new(),
            volume: 1.0,
            pitch: 1.0,
            looping: false,
            spatial: true,
            play_on_spawn: true,
            state: AudioPlayState::Stopped,
            load_mode: AudioLoadMode::Static,
            auto_despawn: false,
            max_distance: 60.0,
        }
    }
}

impl AudioSource {
    pub fn music(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            volume: 0.45,
            pitch: 1.0,
            looping: true,
            spatial: false,
            play_on_spawn: true,
            state: AudioPlayState::Playing,
            load_mode: AudioLoadMode::Streaming,
            auto_despawn: false,
            max_distance: 0.0,
        }
    }

    pub fn one_shot_3d(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            volume: 1.0,
            pitch: 1.0,
            looping: false,
            spatial: true,
            play_on_spawn: true,
            state: AudioPlayState::Playing,
            load_mode: AudioLoadMode::Static,
            auto_despawn: true,
            max_distance: 60.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AudioListener {
    pub active: bool,
}

impl Default for AudioListener {
    fn default() -> Self { Self { active: true } }
}

