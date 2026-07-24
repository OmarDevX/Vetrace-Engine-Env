    fn streaming_data(path: &Path, src: &AudioSource, initial_volume: f32) -> Option<StreamingSoundData<FromFileError>> {
        let mut settings = StreamingSoundSettings::new()
            .playback_rate(PlaybackRate::Factor(src.pitch.max(0.01) as f64))
            .volume(volume_from_amp(initial_volume));
        if src.looping {
            settings = settings.loop_region(std::ops::RangeFull);
        }
        match StreamingSoundData::from_file(path) {
            Ok(data) => Some(data.with_settings(settings)),
            Err(err) => {
                eprintln!("Audio failed to stream {}: {err}", path.display());
                None
            }
        }
    }

    fn has_audio_demand(engine: &Engine) -> bool {
        engine.raw_world().query::<AudioSource>().into_iter().any(|(_, source)| {
            !source.path.trim().is_empty()
                && (source.play_on_spawn || matches!(source.state, AudioPlayState::Playing))
        })
    }

    fn audio_env_disabled() -> bool {
        std::env::var("VETRACE_AUDIO")
            .ok()
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                matches!(value.as_str(), "0" | "off" | "false" | "disabled" | "none")
            })
            .unwrap_or(false)
    }

    fn audio_retry_enabled() -> bool {
        std::env::var("VETRACE_AUDIO_RETRY")
            .ok()
            .map(|value| {
                let value = value.trim().to_ascii_lowercase();
                matches!(value.as_str(), "1" | "on" | "true" | "yes" | "enabled")
            })
            .unwrap_or(false)
    }

    fn linux_audio_preflight_error() -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            let pulse_plugin_exists = [
                "/lib64/alsa-lib/libasound_module_pcm_pulse.so",
                "/usr/lib64/alsa-lib/libasound_module_pcm_pulse.so",
                "/lib/alsa-lib/libasound_module_pcm_pulse.so",
                "/usr/lib/alsa-lib/libasound_module_pcm_pulse.so",
            ]
            .iter()
            .any(|path| std::path::Path::new(path).exists());

            let pipewire_plugin_exists = [
                "/lib64/alsa-lib/libasound_module_pcm_pipewire.so",
                "/usr/lib64/alsa-lib/libasound_module_pcm_pipewire.so",
                "/lib/alsa-lib/libasound_module_pcm_pipewire.so",
                "/usr/lib/alsa-lib/libasound_module_pcm_pipewire.so",
            ]
            .iter()
            .any(|path| std::path::Path::new(path).exists());

            let mut alsa_config_paths = vec![std::path::PathBuf::from("/etc/asound.conf")];
            if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
                alsa_config_paths.push(home.join(".asoundrc"));
            }
            if let Ok(entries) = std::fs::read_dir("/usr/share/alsa/alsa.conf.d") {
                for entry in entries.flatten() {
                    alsa_config_paths.push(entry.path());
                }
            }

            let alsa_config_mentions_pulse = alsa_config_paths
                .iter()
                .filter_map(|path| std::fs::read_to_string(path).ok())
                .any(|text| text.contains("type pulse") || text.contains("pcm.pulse") || text.contains("pulse"));

            if alsa_config_mentions_pulse && !pulse_plugin_exists {
                return Some("ALSA config mentions the Pulse plugin, but libasound_module_pcm_pulse.so is missing for this architecture".to_string());
            }

            if !pulse_plugin_exists && !pipewire_plugin_exists {
                return Some("no ALSA Pulse/PipeWire PCM plugin was found; install alsa-plugins-pulseaudio.x86_64 or pipewire-alsa.x86_64".to_string());
            }
        }
        None
    }

    fn listener_pose(engine: &Engine) -> Option<(Vec3, Quat)> {
        for (entity, listener) in engine.raw_world().query::<AudioListener>() {
            if !listener.active { continue; }
            let position = entity_position(engine, entity);
            let rotation = entity_rotation(engine, entity);
            return Some((position, rotation));
        }
        None
    }

    fn entity_position(engine: &Engine, entity: Entity) -> Vec3 {
        engine.raw_world().get::<GlobalTransform>(entity)
            .map(|transform| transform.translation)
            .or_else(|| engine.raw_world().get::<Transform>(entity).map(|transform| transform.translation))
            .unwrap_or(Vec3::ZERO)
    }

    fn entity_rotation(engine: &Engine, entity: Entity) -> Quat {
        engine.raw_world().get::<GlobalTransform>(entity)
            .map(|transform| transform.rotation)
            .or_else(|| engine.raw_world().get::<Transform>(entity).map(|transform| transform.rotation))
            .unwrap_or(Quat::IDENTITY)
    }

    fn resolve_audio_path(path: &str) -> PathBuf {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() || candidate.exists() {
            return candidate;
        }
        candidate
    }

    fn effective_volume(src: &AudioSource, _source_position: Vec3, _listener_position: Vec3) -> f32 {
        // Kira 0.9 applies spatial attenuation through the emitter distances.
        // Keep per-source volume here and avoid applying distance twice.
        src.volume.max(0.0)
    }

    fn volume_from_amp(amp: f32) -> Volume {
        Volume::Amplitude(amp.max(0.0) as f64)
    }

    fn vec3_to_mint(value: Vec3) -> Vector3<f32> {
        Vector3 { x: value.x, y: value.y, z: value.z }
    }

    fn quat_to_mint(value: Quat) -> Quaternion<f32> {
        Quaternion {
            v: Vector3 { x: value.x, y: value.y, z: value.z },
            s: value.w,
        }
    }
