    impl AudioBackend {
        pub fn initialize(&mut self) {
            // Do not eagerly touch the OS audio device during app startup.
            // Some Linux setups can fail inside ALSA/Pulse/CPAL before the game
            // has even spawned an AudioSource. We initialize lazily on demand.
            if audio_env_disabled() {
                self.last_init_error = Some("disabled by VETRACE_AUDIO".to_string());
                self.soft_disabled = true;
            }
        }

        fn try_initialize(&mut self, log_errors: bool) -> bool {
            if self.enabled() {
                return true;
            }

            if self.soft_disabled && !audio_retry_enabled() {
                return false;
            }

            if audio_env_disabled() {
                self.last_init_error = Some("disabled by VETRACE_AUDIO".to_string());
                self.soft_disabled = true;
                return false;
            }

            if let Some(message) = linux_audio_preflight_error() {
                self.record_init_error(message, log_errors);
                return false;
            }

            self.manager = None;
            self.scene = None;
            self.listener = None;

            let mut manager = match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
                Ok(manager) => manager,
                Err(err) => {
                    self.record_init_error(format!("failed to initialize audio device: {err}"), log_errors);
                    return false;
                }
            };
            let mut scene = match manager.add_spatial_scene(SpatialSceneSettings::default()) {
                Ok(scene) => scene,
                Err(err) => {
                    self.record_init_error(format!("failed to create spatial scene: {err}"), log_errors);
                    return false;
                }
            };
            let listener = match scene.add_listener(
                vec3_to_mint(Vec3::ZERO),
                quat_to_mint(Quat::IDENTITY),
                ListenerSettings::default(),
            ) {
                Ok(listener) => listener,
                Err(err) => {
                    self.record_init_error(format!("failed to create listener: {err}"), log_errors);
                    return false;
                }
            };

            self.manager = Some(manager);
            self.scene = Some(scene);
            self.listener = Some(listener);
            self.init_retry_frames = 0;
            self.last_init_error = None;
            self.soft_disabled = false;
            true
        }

        fn record_init_error(&mut self, message: String, log_errors: bool) {
            let changed = self.last_init_error.as_deref() != Some(message.as_str());
            self.last_init_error = Some(message.clone());
            // Keep this run safe: once the backend fails to initialize, do not
            // repeatedly enter ALSA/CPAL/Kira from the game loop. On broken Linux
            // audio stacks, repeated init/drop attempts can crash inside native
            // audio libraries. Restart the game after fixing the OS audio stack,
            // or set VETRACE_AUDIO_RETRY=1 while debugging hot-plugged devices.
            self.soft_disabled = !audio_retry_enabled();
            self.init_retry_frames = if self.soft_disabled { u32::MAX } else { 300 };
            if log_errors || changed {
                eprintln!("Audio unavailable: {message}");
                if self.soft_disabled {
                    eprintln!("Audio is disabled for this run to avoid unsafe backend retry loops. Restart after fixing OS audio, or set VETRACE_AUDIO_RETRY=1 to retry.");
                } else {
                    eprintln!("Audio will retry while AudioSource entities exist. Set VETRACE_AUDIO=off to force no-op audio.");
                }
            }
        }
    }
