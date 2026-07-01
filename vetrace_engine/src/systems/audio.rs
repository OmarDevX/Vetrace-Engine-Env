use std::collections::HashMap;

use kira::{
    listener::ListenerHandle,
    sound::{
        streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
        PlaybackState,
    },
    track::{SpatialTrackBuilder, SpatialTrackHandle},
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, PlaybackRate, Tween,
};
use mint::{Quaternion, Vector3};

use crate::components::components::{
    AudioClipHandle, AudioPlayState, AudioSource, GlobalTransform, Transform,
};
use crate::ecs::Entity;
use crate::engine::engine::Engine;
use crate::Behaviour;

fn amp_to_db(amp: f32) -> Decibels {
    if amp <= 0.0 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * amp.log10())
    }
}

pub struct AudioSystem {
    manager: Option<AudioManager<DefaultBackend>>,
    listener: Option<ListenerHandle>,
    handles: HashMap<Entity, PlayingHandle>,
}

struct PlayingHandle {
    track: Option<SpatialTrackHandle>,
    handle: SoundHandle,
    clip: Option<AudioClipHandle>,
}

enum SoundHandle {
    Streaming(StreamingSoundHandle<kira::sound::FromFileError>),
}

impl SoundHandle {
    fn state(&self) -> PlaybackState {
        match self {
            SoundHandle::Streaming(h) => h.state(),
        }
    }

    fn set_volume(&mut self, volume: Decibels, tween: Tween) {
        if let SoundHandle::Streaming(h) = self {
            h.set_volume(volume, tween);
        }
    }

    fn set_playback_rate(&mut self, rate: PlaybackRate, tween: Tween) {
        if let SoundHandle::Streaming(h) = self {
            h.set_playback_rate(rate, tween);
        }
    }

    fn pause(&mut self, tween: Tween) {
        if let SoundHandle::Streaming(h) = self {
            h.pause(tween);
        }
    }

    fn stop(&mut self, tween: Tween) {
        if let SoundHandle::Streaming(h) = self {
            h.stop(tween);
        }
    }
}
impl AudioSystem {
    pub fn new() -> Self {
        let (manager, listener) = match AudioManager::new(AudioManagerSettings::default()) {
            Ok(mut manager) => {
                match manager.add_listener(
                    Vector3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Quaternion {
                        v: Vector3 {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        s: 1.0,
                    },
                ) {
                    Ok(listener) => (Some(manager), Some(listener)),
                    Err(err) => {
                        eprintln!("Audio disabled: failed to create listener: {err}");
                        (None, None)
                    }
                }
            }
            Err(err) => {
                eprintln!("Audio disabled: failed to initialize audio device: {err}");
                (None, None)
            }
        };
        Self {
            manager,
            listener,
            handles: HashMap::new(),
        }
    }

    fn play_source(&mut self, entity: Entity, src: &AudioSource, position: Vector3<f32>) {
        let Some(manager) = self.manager.as_mut() else {
            return;
        };
        if let Some(path) = &src.clip {
            if let Ok(data) = StreamingSoundData::from_file(path) {
                let mut settings = StreamingSoundSettings::new()
                    .playback_rate(PlaybackRate(src.pitch as f64))
                    .volume(amp_to_db(src.volume));
                if src.loop_ {
                    settings = settings.loop_region(std::ops::RangeFull);
                }
                let data = data.with_settings(settings);
                if src.spatial {
                    let Some(listener) = self.listener.as_ref() else {
                        return;
                    };
                    if let Ok(mut track) = manager.add_spatial_sub_track(
                        listener.id(),
                        position,
                        SpatialTrackBuilder::new(),
                    ) {
                        if let Ok(handle) = track.play(data) {
                            self.handles.insert(
                                entity,
                                PlayingHandle {
                                    track: Some(track),
                                    handle: SoundHandle::Streaming(handle),
                                    clip: src.clip.clone(),
                                },
                            );
                        }
                    }
                } else {
                    if let Ok(handle) = manager.play(data) {
                        self.handles.insert(
                            entity,
                            PlayingHandle {
                                track: None,
                                handle: SoundHandle::Streaming(handle),
                                clip: src.clip.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
}

impl Behaviour for AudioSystem {
    fn start(&mut self, engine: &mut Engine) {
        let Some(listener) = self.listener.as_mut() else {
            return;
        };
        // Update the listener to match the active camera at startup.
        let cam = engine.active_camera_info();
        listener.set_position(
            Vector3 {
                x: cam.position.x,
                y: cam.position.y,
                z: cam.position.z,
            },
            Tween::default(),
        );
        listener.set_orientation(
            Quaternion {
                v: Vector3 {
                    x: cam.orientation.x,
                    y: cam.orientation.y,
                    z: cam.orientation.z,
                },
                s: cam.orientation.w,
            },
            Tween::default(),
        );

        // `GlobalTransform` components are created by `TransformSyncSystem`
        // during the first update, so they may not exist yet when `start` is
        // called. Use the local `Transform` so `play_on_start` works immediately.
        for (entity, src, transform) in engine.world.query2_mut::<AudioSource, Transform>() {
            if src.play_on_start {
                src.state = AudioPlayState::Playing;
                let pos = Vector3 {
                    x: transform.position[0],
                    y: transform.position[1],
                    z: transform.position[2],
                };
                self.play_source(entity, src, pos);
            }
        }
    }

    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        let Some(listener) = self.listener.as_mut() else {
            return;
        };
        // Update listener position and orientation each frame so spatial audio
        // responds to camera movement.
        let cam = engine.active_camera_info();
        listener.set_position(
            Vector3 {
                x: cam.position.x,
                y: cam.position.y,
                z: cam.position.z,
            },
            Tween::default(),
        );
        listener.set_orientation(
            Quaternion {
                v: Vector3 {
                    x: cam.orientation.x,
                    y: cam.orientation.y,
                    z: cam.orientation.z,
                },
                s: cam.orientation.w,
            },
            Tween::default(),
        );

        // Stop sounds for entities that no longer have an `AudioSource` component.
        let mut to_remove = Vec::new();
        for (&entity, playing) in self.handles.iter_mut() {
            if !engine.world.has::<AudioSource>(entity) {
                playing.handle.stop(Tween::default());
                to_remove.push(entity);
            }
        }
        for entity in to_remove {
            self.handles.remove(&entity);
        }

        for (entity, src, transform) in engine.world.query2_mut::<AudioSource, GlobalTransform>() {
            match src.state {
                AudioPlayState::Playing => {
                    let pos = Vector3 {
                        x: transform.position[0],
                        y: transform.position[1],
                        z: transform.position[2],
                    };
                    if let Some(playing) = self.handles.get_mut(&entity) {
                        // Restart if the clip changed
                        if playing.clip != src.clip {
                            playing.handle.stop(Tween::default());
                            self.handles.remove(&entity);
                            self.play_source(entity, src, pos);
                            continue;
                        }

                        playing
                            .handle
                            .set_volume(amp_to_db(src.volume), Tween::default());
                        playing
                            .handle
                            .set_playback_rate(PlaybackRate(src.pitch as f64), Tween::default());
                        if let Some(t) = &mut playing.track {
                            t.set_position(pos, Tween::default());
                        }
                        if !src.loop_ && playing.handle.state() == PlaybackState::Stopped {
                            src.state = AudioPlayState::Stopped;
                            self.handles.remove(&entity);
                        }
                    } else {
                        self.play_source(entity, src, pos);
                    }
                }
                AudioPlayState::Paused => {
                    if let Some(playing) = self.handles.get_mut(&entity) {
                        playing.handle.pause(Tween::default());
                    }
                }
                AudioPlayState::Stopped => {
                    if let Some(mut playing) = self.handles.remove(&entity) {
                        playing.handle.stop(Tween::default());
                    }
                }
            }
        }
    }
}
