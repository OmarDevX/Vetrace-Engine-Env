    impl AudioBackend {
        pub fn update(&mut self, engine: &mut Engine) {
            if audio_env_disabled() {
                self.stop_all();
                self.soft_disabled = true;
                self.last_init_error = Some("disabled by VETRACE_AUDIO".to_string());
                return;
            }

            if !self.enabled() {
                if self.soft_disabled && !audio_retry_enabled() {
                    return;
                }
                if self.init_retry_frames > 0 {
                    self.init_retry_frames = self.init_retry_frames.saturating_sub(1);
                    return;
                }
                if has_audio_demand(engine) && !self.try_initialize(false) {
                    return;
                }
            }

            if !self.enabled() {
                return;
            }

            self.update_listener(engine);
            self.stop_removed_sources(engine);
            let listener_position = listener_pose(engine).map(|(position, _)| position).unwrap_or(Vec3::ZERO);

            let entities = engine.raw_world().query::<AudioSource>()
                .into_iter()
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>();

            let mut despawn_after_update = Vec::new();
            for entity in entities {
                let position = entity_position(engine, entity);
                let Some(src) = engine.raw_world_mut().get_mut::<AudioSource>(entity) else { continue; };
                if src.play_on_spawn && src.state == AudioPlayState::Stopped {
                    src.state = AudioPlayState::Playing;
                }

                match src.state {
                    AudioPlayState::Playing => {
                        let restart = self.handles.get(&entity).map(|playing| {
                            playing.path != src.path
                                || playing.load_mode != src.load_mode
                                || playing.spatial != src.spatial
                                || playing.looping != src.looping
                        }).unwrap_or(false);
                        if restart {
                            if let Some(mut old) = self.handles.remove(&entity) {
                                old.handle.stop(Tween::default());
                            }
                        }

                        if let Some(playing) = self.handles.get_mut(&entity) {
                            playing.handle.set_volume(volume_from_amp(effective_volume(src, position, listener_position)), Tween::default());
                            playing.handle.set_playback_rate(PlaybackRate::Factor(src.pitch.max(0.01) as f64), Tween::default());
                            if let Some(emitter) = &mut playing.emitter {
                                emitter.set_position(vec3_to_mint(position), Tween::default());
                            }
                            if !src.looping && playing.handle.state() == PlaybackState::Stopped {
                                src.state = AudioPlayState::Stopped;
                                if playing.auto_despawn || src.auto_despawn {
                                    despawn_after_update.push(entity);
                                }
                            }
                        } else {
                            self.play_source(entity, src, position, listener_position);
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

            for entity in despawn_after_update {
                self.handles.remove(&entity);
                if engine.raw_world().is_alive(entity) {
                    engine.raw_world_mut().despawn(entity);
                }
            }
        }

        fn stop_all(&mut self) {
            for (_, mut playing) in self.handles.drain() {
                playing.handle.stop(Tween::default());
            }
            self.listener = None;
            self.scene = None;
            self.manager = None;
        }

        fn update_listener(&mut self, engine: &Engine) {
            let Some(listener) = self.listener.as_mut() else { return; };
            let (position, rotation) = listener_pose(engine).unwrap_or((Vec3::ZERO, Quat::IDENTITY));
            listener.set_position(vec3_to_mint(position), Tween::default());
            listener.set_orientation(quat_to_mint(rotation), Tween::default());
        }

        fn stop_removed_sources(&mut self, engine: &Engine) {
            let mut to_remove = Vec::new();
            for (&entity, playing) in self.handles.iter_mut() {
                if !engine.raw_world().is_alive(entity) || !engine.raw_world().has::<AudioSource>(entity) {
                    playing.handle.stop(Tween::default());
                    to_remove.push(entity);
                }
            }
            for entity in to_remove {
                self.handles.remove(&entity);
            }
        }
    }
