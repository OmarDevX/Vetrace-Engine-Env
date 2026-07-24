    impl AudioBackend {
        fn play_source(&mut self, entity: Entity, src: &AudioSource, position: Vec3, listener_position: Vec3) {
            if src.path.trim().is_empty() {
                return;
            }
            let path = resolve_audio_path(&src.path);
            if !path.exists() {
                let key = path.display().to_string();
                if !self.warned_missing.contains_key(&key) {
                    eprintln!("Audio asset missing: {}", path.display());
                    self.warned_missing.insert(key, true);
                }
                return;
            }

            let initial_volume = effective_volume(src, position, listener_position);
            let emitter = if src.spatial {
                let Some(scene) = self.scene.as_mut() else { return; };
                let settings = EmitterSettings::default()
                    .distances((1.0, src.max_distance.max(1.01)))
                    .attenuation_function(Some(Easing::Linear))
                    .enable_spatialization(true)
                    .persist_until_sounds_finish(true);
                match scene.add_emitter(vec3_to_mint(position), settings) {
                    Ok(emitter) => Some(emitter),
                    Err(err) => {
                        eprintln!("Audio failed to create spatial emitter for {}: {err}", path.display());
                        return;
                    }
                }
            } else {
                None
            };

            let handle = match src.load_mode {
                AudioLoadMode::Static => {
                    let Some(mut data) = self.load_static(path.as_path(), src, initial_volume) else { return; };
                    if let Some(emitter) = emitter.as_ref() {
                        data = data.output_destination(emitter);
                    }
                    let Some(manager) = self.manager.as_mut() else { return; };
                    match manager.play(data) {
                        Ok(handle) => PlayingHandle {
                            emitter,
                            handle: SoundHandle::Static(handle),
                            path: src.path.clone(),
                            load_mode: src.load_mode,
                            spatial: src.spatial,
                            looping: src.looping,
                            auto_despawn: src.auto_despawn,
                        },
                        Err(err) => {
                            eprintln!("Audio failed to play {}: {err}", path.display());
                            return;
                        }
                    }
                }
                AudioLoadMode::Streaming => {
                    let Some(mut data) = streaming_data(path.as_path(), src, initial_volume) else { return; };
                    if let Some(emitter) = emitter.as_ref() {
                        data = data.output_destination(emitter);
                    }
                    let Some(manager) = self.manager.as_mut() else { return; };
                    match manager.play(data) {
                        Ok(handle) => PlayingHandle {
                            emitter,
                            handle: SoundHandle::Streaming(handle),
                            path: src.path.clone(),
                            load_mode: src.load_mode,
                            spatial: src.spatial,
                            looping: src.looping,
                            auto_despawn: src.auto_despawn,
                        },
                        Err(err) => {
                            eprintln!("Audio failed to play {}: {err}", path.display());
                            return;
                        }
                    }
                }
            };
            self.handles.insert(entity, handle);
        }

        fn load_static(&mut self, path: &Path, src: &AudioSource, initial_volume: f32) -> Option<StaticSoundData> {
            let key = path.display().to_string();
            if !self.static_cache.contains_key(&key) {
                match StaticSoundData::from_file(path) {
                    Ok(data) => {
                        self.static_cache.insert(key.clone(), data);
                    }
                    Err(err) => {
                        eprintln!("Audio failed to load {}: {err}", path.display());
                        return None;
                    }
                }
            }
            let data = self.static_cache.get(&key)?.clone();
            let mut settings = StaticSoundSettings::new()
                .playback_rate(PlaybackRate::Factor(src.pitch.max(0.01) as f64))
                .volume(volume_from_amp(initial_volume));
            if src.looping {
                settings = settings.loop_region(std::ops::RangeFull);
            }
            Some(data.with_settings(settings))
        }
    }
