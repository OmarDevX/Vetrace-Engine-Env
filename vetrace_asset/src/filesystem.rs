use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::{AssetError, AssetId, AssetResult};

pub(crate) fn unique_destination(directory: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let candidate = directory.join(file_name);
    if !candidate.exists() { return candidate; }
    let path = Path::new(file_name);
    let stem = path.file_stem().unwrap_or(file_name).to_string_lossy();
    let extension = path.extension().map(|value| value.to_string_lossy().to_string());
    for index in 1..10_000 {
        let name = match &extension {
            Some(extension) => format!("{stem}-{index}.{extension}"),
            None => format!("{stem}-{index}"),
        };
        let candidate = directory.join(name);
        if !candidate.exists() { return candidate; }
    }
    directory.join(format!("{}-{}", stem, AssetId::new()))
}

pub(crate) fn write_json_atomic(path: &Path, value: &impl Serialize) -> AssetResult<()> {
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = fs::File::create(&temporary)
        .map_err(|error| AssetError::io("create import metadata", &temporary, error))?;
    file.write_all(&bytes)
        .map_err(|error| AssetError::io("write import metadata", &temporary, error))?;
    file.sync_all()
        .map_err(|error| AssetError::io("sync import metadata", &temporary, error))?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| AssetError::io("replace import metadata", path, error))?;
    }
    fs::rename(&temporary, path).map_err(|error| AssetError::io("replace import metadata", path, error))
}

pub(crate) fn clear_directory(path: &Path) -> AssetResult<()> {
    if !path.exists() { return Ok(()); }
    for entry in fs::read_dir(path).map_err(|error| AssetError::io("read cache directory", path, error))? {
        let entry = entry.map_err(|error| AssetError::io("read cache entry", path, error))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path)
                .map_err(|error| AssetError::io("remove cache directory", &entry_path, error))?;
        } else {
            fs::remove_file(&entry_path)
                .map_err(|error| AssetError::io("remove cache file", &entry_path, error))?;
        }
    }
    Ok(())
}

pub(crate) fn directory_size(path: &Path) -> (usize, u64) {
    let mut files = 0;
    let mut bytes = 0;
    let Ok(entries) = fs::read_dir(path) else { return (files, bytes); };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let nested = directory_size(&path);
            files += nested.0;
            bytes += nested.1;
        } else if let Ok(metadata) = entry.metadata() {
            files += 1;
            bytes += metadata.len();
        }
    }
    (files, bytes)
}

pub(crate) fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
        .as_millis().min(u64::MAX as u128) as u64
}
