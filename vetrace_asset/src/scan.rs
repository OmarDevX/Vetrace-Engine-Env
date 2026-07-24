use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vetrace_project::{ProjectPath, ProjectPaths};

use crate::{AssetError, AssetResult};

#[derive(Clone, Debug)]
pub(crate) struct DiscoveredFile {
    pub source: ProjectPath,
    pub absolute: PathBuf,
    pub hash: String,
    pub size: u64,
    pub modified_unix_ms: u64,
}

pub(crate) fn discover_files(paths: &ProjectPaths) -> AssetResult<Vec<DiscoveredFile>> {
    let mut files = Vec::new();
    visit_assets(paths, paths.assets(), &mut files)?;
    files.sort_by(|left, right| left.source.cmp(&right.source));
    Ok(files)
}

fn visit_assets(paths: &ProjectPaths, directory: &Path, output: &mut Vec<DiscoveredFile>) -> AssetResult<()> {
    let entries = fs::read_dir(directory)
        .map_err(|error| AssetError::io("scan asset directory", directory, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| AssetError::io("read asset directory entry", directory, error))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| AssetError::io("read asset metadata", &path, error))?;
        if metadata.file_type().is_symlink() { continue; }
        if metadata.is_dir() {
            visit_assets(paths, &path, output)?;
        } else if metadata.is_file() {
            let source = paths.to_project_path(&path)
                .map_err(|error| AssetError::InvalidPath(error.to_string()))?;
            output.push(DiscoveredFile {
                source,
                absolute: path.clone(),
                hash: hash_file(&path)?,
                size: metadata.len(),
                modified_unix_ms: metadata.modified().ok().map(system_time_ms).unwrap_or(0),
            });
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> AssetResult<String> {
    let mut file = fs::File::open(path).map_err(|error| AssetError::io("open asset for hashing", path, error))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| AssetError::io("hash asset", path, error))?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn system_time_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis().min(u64::MAX as u128) as u64
}
