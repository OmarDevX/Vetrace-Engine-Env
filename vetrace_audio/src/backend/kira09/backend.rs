    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    use glam::{Quat, Vec3};
    use kira::{
        manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
        sound::{
            FromFileError, PlaybackRate, PlaybackState,
            static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
            streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
        },
        spatial::{
            emitter::{EmitterHandle, EmitterSettings},
            listener::{ListenerHandle, ListenerSettings},
            scene::{SpatialSceneHandle, SpatialSceneSettings},
        },
        tween::{Easing, Tween},
        Volume,
    };
    use mint::{Quaternion, Vector3};
    use vetrace_core::{Engine, Entity, GlobalTransform, Transform};

    use crate::components::{AudioListener, AudioLoadMode, AudioPlayState, AudioSource};

    pub struct AudioBackend {
        manager: Option<AudioManager<DefaultBackend>>,
        scene: Option<SpatialSceneHandle>,
        listener: Option<ListenerHandle>,
        handles: HashMap<Entity, PlayingHandle>,
        static_cache: HashMap<String, StaticSoundData>,
        warned_missing: HashMap<String, bool>,
        init_retry_frames: u32,
        last_init_error: Option<String>,
        soft_disabled: bool,
    }

    struct PlayingHandle {
        emitter: Option<EmitterHandle>,
        handle: SoundHandle,
        path: String,
        load_mode: AudioLoadMode,
        spatial: bool,
        looping: bool,
        auto_despawn: bool,
    }

    enum SoundHandle {
        Static(StaticSoundHandle),
        Streaming(StreamingSoundHandle<FromFileError>),
    }

    impl SoundHandle {
        fn state(&self) -> PlaybackState {
            match self {
                SoundHandle::Static(handle) => handle.state(),
                SoundHandle::Streaming(handle) => handle.state(),
            }
        }

        fn set_volume(&mut self, volume: Volume, tween: Tween) {
            match self {
                SoundHandle::Static(handle) => handle.set_volume(volume, tween),
                SoundHandle::Streaming(handle) => handle.set_volume(volume, tween),
            }
        }

        fn set_playback_rate(&mut self, rate: PlaybackRate, tween: Tween) {
            match self {
                SoundHandle::Static(handle) => handle.set_playback_rate(rate, tween),
                SoundHandle::Streaming(handle) => handle.set_playback_rate(rate, tween),
            }
        }

        fn pause(&mut self, tween: Tween) {
            match self {
                SoundHandle::Static(handle) => handle.pause(tween),
                SoundHandle::Streaming(handle) => handle.pause(tween),
            }
        }

        fn stop(&mut self, tween: Tween) {
            match self {
                SoundHandle::Static(handle) => handle.stop(tween),
                SoundHandle::Streaming(handle) => handle.stop(tween),
            }
        }
    }

    impl AudioBackend {
        pub fn new() -> Self {
            Self {
                manager: None,
                scene: None,
                listener: None,
                handles: HashMap::new(),
                static_cache: HashMap::new(),
                warned_missing: HashMap::new(),
                init_retry_frames: 0,
                last_init_error: None,
                soft_disabled: false,
            }
        }

        pub fn name(&self) -> &'static str { "kira" }
        pub fn enabled(&self) -> bool { self.manager.is_some() && self.scene.is_some() && self.listener.is_some() }
    }
