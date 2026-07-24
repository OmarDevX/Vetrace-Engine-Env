use super::*;

pub fn default_template_root() -> PathBuf {
    if let Some(path) = std::env::var_os("VETRACE_TEMPLATE_HOME") {
        return PathBuf::from(path);
    }
    let home = std::env::var_os(if cfg!(windows) { "LOCALAPPDATA" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if cfg!(windows) {
        home.join("Vetrace").join("templates")
    } else {
        home.join(".vetrace").join("templates")
    }
}

pub(super) fn extract_safe(archive: &mut ZipArchive<fs::File>, destination: &Path) -> BuildResult<()> {
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(BuildError::Validation(format!("unsafe path in template bundle: {}", entry.name())));
        };
        let output = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&output)
                .map_err(|error| BuildError::io("create template bundle directory", &output, error))?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| BuildError::io("create template bundle directory", parent, error))?;
        }
        let mut file = fs::File::create(&output)
            .map_err(|error| BuildError::io("extract template bundle", &output, error))?;
        std::io::copy(&mut entry, &mut file)
            .map_err(|error| BuildError::io("extract template bundle", &output, error))?;
    }
    Ok(())
}

pub(super) fn safe_join(root: &Path, relative: &str) -> BuildResult<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute() || relative.components().any(|component| matches!(component, std::path::Component::ParentDir)) {
        return Err(BuildError::Validation(format!("unsafe template bundle path '{relative:?}'")));
    }
    Ok(root.join(relative))
}

pub(super) fn sanitize_template_id(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            output.push(character);
        } else if !output.ends_with('-') {
            output.push('-');
        }
    }
    let value = output.trim_matches('-').chars().take(96).collect::<String>();
    if value.is_empty() { "player-template".to_owned() } else { value }
}

pub(super) fn copy_executable_permissions(source: &Path, destination: &Path) -> BuildResult<()> {
    let permissions = fs::metadata(source)
        .map_err(|error| BuildError::io("read player-template permissions", source, error))?
        .permissions();
    fs::set_permissions(destination, permissions)
        .map_err(|error| BuildError::io("set player-template permissions", destination, error))
}



pub(super) fn validate_catalog_url(url: &str) -> BuildResult<()> {
    let allowed = url.starts_with("https://")
        || url.starts_with("http://localhost/")
        || url.starts_with("http://127.0.0.1/")
        || url.starts_with("http://[::1]/");
    if allowed {
        Ok(())
    } else {
        Err(BuildError::Validation(
            "template URL must use HTTPS; plain HTTP is allowed only for localhost".into(),
        ))
    }
}

#[cfg(feature = "template_download")]
pub(super) fn validate_http_url(url: &str) -> BuildResult<()> { validate_catalog_url(url) }

pub(super) fn validate_template_metadata_shape(metadata: &PlayerTemplateMetadata) -> BuildResult<()> {
    if metadata.format_version != PLAYER_TEMPLATE_METADATA_FORMAT_VERSION {
        return Err(BuildError::Validation(format!(
            "unsupported player-template metadata format {}; expected {}",
            metadata.format_version, PLAYER_TEMPLATE_METADATA_FORMAT_VERSION,
        )));
    }
    if metadata.engine_version.trim().is_empty() {
        return Err(BuildError::Validation("player-template engine version is empty".into()));
    }
    if metadata.vpak_format_version != VPAK_FORMAT_VERSION {
        return Err(BuildError::Validation(format!(
            "player template supports VPAK format {}, but this build supports format {}",
            metadata.vpak_format_version, VPAK_FORMAT_VERSION,
        )));
    }
    if metadata.features.iter().any(|feature| feature.trim().is_empty()) {
        return Err(BuildError::Validation("player-template metadata contains an empty feature name".into()));
    }
    Ok(())
}

#[cfg(feature = "template_download")]
pub(super) fn download_to_file(url: &str, destination: &Path, expected_bytes: Option<u64>) -> BuildResult<()> {
    validate_http_url(url)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| BuildError::io("create template download directory", parent, error))?;
    }
    let temporary = destination.with_extension(format!(
        "{}.download",
        destination.extension().and_then(|value| value.to_str()).unwrap_or("tmp"),
    ));
    let _ = fs::remove_file(&temporary);

    let mut attempts = Vec::new();
    let downloaded = if let Some(custom) = std::env::var_os("VETRACE_HTTP_DOWNLOADER") {
        run_downloader(std::process::Command::new(custom).arg(url).arg(&temporary), &mut attempts)
    } else {
        let curl = run_downloader(
            std::process::Command::new("curl")
                .args(["--fail", "--location", "--silent", "--show-error", "--output"])
                .arg(&temporary)
                .arg(url),
            &mut attempts,
        );
        if curl {
            true
        } else {
            let wget = run_downloader(
                std::process::Command::new("wget")
                    .args(["--quiet", "--output-document"])
                    .arg(&temporary)
                    .arg(url),
                &mut attempts,
            );
            if wget {
                true
            } else if cfg!(windows) {
                let script = format!(
                    "$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
                    powershell_quote(url),
                    powershell_quote(&temporary.to_string_lossy()),
                );
                run_downloader(
                    std::process::Command::new("powershell")
                        .args(["-NoProfile", "-NonInteractive", "-Command"])
                        .arg(script),
                    &mut attempts,
                )
            } else {
                false
            }
        }
    };

    if !downloaded || !temporary.is_file() {
        let _ = fs::remove_file(&temporary);
        return Err(BuildError::Validation(format!(
            "failed to download player template from '{url}'. Install curl or wget, or set VETRACE_HTTP_DOWNLOADER. Attempts: {}",
            attempts.join("; "),
        )));
    }
    let actual = fs::metadata(&temporary)
        .map_err(|error| BuildError::io("inspect template download", &temporary, error))?
        .len();
    if let Some(expected) = expected_bytes {
        if actual != expected {
            let _ = fs::remove_file(&temporary);
            return Err(BuildError::Validation(format!(
                "player-template download size mismatch: expected {expected} bytes, received {actual}",
            )));
        }
    }
    #[cfg(windows)]
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| BuildError::io("replace template download", destination, error))?;
    }
    fs::rename(&temporary, destination)
        .map_err(|error| BuildError::io("commit template download", destination, error))
}

#[cfg(feature = "template_download")]
pub(super) fn run_downloader(command: &mut std::process::Command, attempts: &mut Vec<String>) -> bool {
    let description = format!("{command:?}");
    match command.status() {
        Ok(status) if status.success() => true,
        Ok(status) => {
            attempts.push(format!("{description} exited with {status}"));
            false
        }
        Err(error) => {
            attempts.push(format!("{description}: {error}"));
            false
        }
    }
}

#[cfg(feature = "template_download")]
pub(super) fn powershell_quote(value: &str) -> String { value.replace('\\', "\\\\").replace('\'', "''") }

pub(super) fn blake3_file(path: &Path) -> BuildResult<String> {
    let mut file = fs::File::open(path)
        .map_err(|error| BuildError::io("open file for BLAKE3", path, error))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)
            .map_err(|error| BuildError::io("read file for BLAKE3", path, error))?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub(super) fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().try_into().unwrap_or(u64::MAX)
}
