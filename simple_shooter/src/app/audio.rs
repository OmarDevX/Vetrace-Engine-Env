use super::*;

#[cfg(feature = "audio")]
pub(crate) const BACKGROUND_MUSIC_ASSET: &str = "backmusic.mp3";
#[cfg(feature = "audio")]

#[cfg(feature = "audio")]
pub(crate) fn spawn_shooter_audio(engine: &mut Engine) {
    let music_path = resolve_simple_shooter_asset_path(BACKGROUND_MUSIC_ASSET);
    if music_path.exists() {
        engine
            .spawn_actor("Background Music")
            .with(AudioSource::music(asset_path_string(music_path)))
            .tag("audio")
            .tag("music")
            .source("simple_shooter")
            .build();
    } else {
        eprintln!("Simple Shooter audio: missing `{}`; background music disabled", music_path.display());
    }

    engine
        .spawn_actor("Audio Listener")
        .with(ShooterAudioListener)
        .with(AudioListener::default())
        .tag("audio")
        .tag("listener")
        .source("simple_shooter")
        .build();
}

#[cfg(not(feature = "audio"))]
pub(crate) fn spawn_shooter_audio(_engine: &mut Engine) {}

#[cfg(feature = "audio")]
pub(crate) fn spawn_shoot_sound(engine: &mut Engine, weapon_id: &str, position: Vec3) {
    let config = weapon_definition(engine, weapon_id);
    let path = resolve_simple_shooter_asset_path(&config.sound.path);
    if !path.exists() {
        // Do not spam every shot when the uploaded zip does not contain the user asset.
        return;
    }

    let mut source = AudioSource::one_shot_3d(asset_path_string(path));
    let master_volume = engine.get_resource::<ShooterGameSettings>().map(|settings| settings.master_volume).unwrap_or(1.0);
    source.volume = config.sound.volume * master_volume;
    source.max_distance = config.sound.max_distance;
    engine
        .spawn_actor("Shoot Sound")
        .with(Transform { translation: position, ..Transform::default() })
        .with(source)
        .tag("audio")
        .tag("sfx")
        .source("simple_shooter")
        .build();
}

#[cfg(not(feature = "audio"))]
pub(crate) fn spawn_shoot_sound(_engine: &mut Engine, _weapon_id: &str, _position: Vec3) {}

#[cfg(feature = "audio")]
pub(crate) fn sync_audio_listener_to_camera(engine: &mut Engine) {
    let Some(camera) = engine.get_resource::<Camera>().cloned() else { return; };
    let rotation = camera_listener_rotation(camera.position, camera.target, camera.up);
    let listeners = engine.actors_with::<ShooterAudioListener>()
        .into_iter()
        .map(|(actor, _)| actor)
        .collect::<Vec<_>>();

    for actor in listeners {
        if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
            transform.translation = camera.position;
            transform.rotation = rotation;
            transform.scale = Vec3::ONE;
        }
    }
}

#[cfg(not(feature = "audio"))]
pub(crate) fn sync_audio_listener_to_camera(_engine: &mut Engine) {}

#[cfg(feature = "audio")]
pub(crate) fn camera_listener_rotation(position: Vec3, target: Vec3, up: Vec3) -> Quat {
    let forward = normalize_or(target - position, Vec3::NEG_Z);
    let mut right = normalize_or(forward.cross(up), Vec3::X);
    if right.length_squared() < 1.0e-6 {
        right = Vec3::X;
    }
    let corrected_up = normalize_or(right.cross(forward), Vec3::Y);

    // Kira 0.9 defines an unrotated listener as facing local -Z, with +X to
    // the right and +Y up. Build a basis where local -Z follows the camera
    // look vector, otherwise front/back spatial audio is reversed.
    Quat::from_mat3(&Mat3::from_cols(right, corrected_up, -forward))
}

#[cfg(feature = "audio")]
pub(crate) fn normalize_or(v: Vec3, fallback: Vec3) -> Vec3 {
    if v.length_squared() > 1.0e-8 {
        v.normalize()
    } else {
        fallback
    }
}

#[cfg(feature = "audio")]
pub(crate) fn asset_path_string(path: std::path::PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
