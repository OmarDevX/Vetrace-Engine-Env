use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{AssetError, AssetResult};

#[derive(Clone, Debug, Default)]
pub struct AssetChangeBatch {
    pub paths: Vec<PathBuf>,
    pub rescan_required: bool,
    pub errors: Vec<String>,
}

impl AssetChangeBatch {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty() && !self.rescan_required && self.errors.is_empty()
    }
}

pub struct AssetWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
    root: PathBuf,
}

impl AssetWatcher {
    pub fn new(assets_root: impl AsRef<Path>) -> AssetResult<Self> {
        let root = assets_root.as_ref().to_path_buf();
        let (sender, receiver) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher = notify::recommended_watcher(move |result| {
            let _ = sender.send(result);
        }).map_err(|error| AssetError::Watch(error.to_string()))?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .map_err(|error| AssetError::Watch(error.to_string()))?;
        Ok(Self { _watcher: watcher, receiver, root })
    }

    pub fn root(&self) -> &Path { &self.root }

    pub fn drain(&self) -> AssetChangeBatch {
        let mut paths = BTreeSet::new();
        let mut errors = Vec::new();
        let mut rescan_required = false;
        while let Ok(result) = self.receiver.try_recv() {
            match result {
                Ok(event) => {
                    if event.paths.is_empty() { rescan_required = true; }
                    for path in event.paths {
                        if path.starts_with(&self.root) { paths.insert(path); }
                    }
                }
                Err(error) => {
                    errors.push(error.to_string());
                    rescan_required = true;
                }
            }
        }
        AssetChangeBatch { paths: paths.into_iter().collect(), rescan_required, errors }
    }
}
