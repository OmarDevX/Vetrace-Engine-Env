use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;

use serde::{Deserialize, Serialize};
use vetrace_project::{ProjectPath, ProjectPaths};

use crate::{AssetDiagnostic, AssetError, AssetId, AssetRecord, AssetResult};

pub const ASSET_DATABASE_FORMAT_VERSION: u32 = 1;
pub const ASSET_DATABASE_PATH: &str = ".vetrace/asset_db.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetDatabase {
    pub format_version: u32,
    #[serde(default)]
    pub records: BTreeMap<AssetId, AssetRecord>,
    #[serde(default)]
    pub diagnostics: Vec<AssetDiagnostic>,
}

impl Default for AssetDatabase {
    fn default() -> Self {
        Self {
            format_version: ASSET_DATABASE_FORMAT_VERSION,
            records: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl AssetDatabase {
    pub fn load(paths: &ProjectPaths) -> AssetResult<Self> {
        let path = paths.root().join(ASSET_DATABASE_PATH);
        if !path.exists() { return Ok(Self::default()); }
        let bytes = fs::read(&path).map_err(|error| AssetError::io("read asset database", &path, error))?;
        let database: Self = serde_json::from_slice(&bytes)?;
        if database.format_version > ASSET_DATABASE_FORMAT_VERSION {
            return Err(AssetError::Database(format!(
                "database format {} is newer than supported format {}",
                database.format_version, ASSET_DATABASE_FORMAT_VERSION
            )));
        }
        Ok(database)
    }

    pub fn save(&self, paths: &ProjectPaths) -> AssetResult<()> {
        let path = paths.root().join(ASSET_DATABASE_PATH);
        let parent = path.parent().ok_or_else(|| AssetError::Database("database path has no parent".into()))?;
        fs::create_dir_all(parent).map_err(|error| AssetError::io("create asset metadata directory", parent, error))?;
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(self)?;
        {
            let mut file = fs::File::create(&temporary)
                .map_err(|error| AssetError::io("create temporary asset database", &temporary, error))?;
            file.write_all(&bytes)
                .map_err(|error| AssetError::io("write temporary asset database", &temporary, error))?;
            file.sync_all()
                .map_err(|error| AssetError::io("sync temporary asset database", &temporary, error))?;
        }
        if let Err(error) = fs::rename(&temporary, &path) {
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|remove_error| AssetError::io("replace asset database", &path, remove_error))?;
                fs::rename(&temporary, &path)
                    .map_err(|rename_error| AssetError::io("replace asset database", &path, rename_error))?;
            } else {
                return Err(AssetError::io("replace asset database", &path, error));
            }
        }
        Ok(())
    }

    pub fn record(&self, id: AssetId) -> Option<&AssetRecord> { self.records.get(&id) }
    pub fn record_mut(&mut self, id: AssetId) -> Option<&mut AssetRecord> { self.records.get_mut(&id) }

    pub fn by_source(&self, source: &ProjectPath) -> Option<&AssetRecord> {
        self.records.values().find(|record| &record.source == source)
    }

    pub fn id_by_source(&self, source: &ProjectPath) -> Option<AssetId> {
        self.by_source(source).map(|record| record.id)
    }

    pub fn current_sources(&self) -> BTreeSet<&ProjectPath> {
        self.records.values().map(|record| &record.source).collect()
    }

    pub fn imported_directory(id: AssetId) -> String { format!(".vetrace/imported/{id}") }

    pub fn metadata_path(id: AssetId) -> Result<ProjectPath, AssetError> {
        ProjectPath::new(format!("{}/import.json", Self::imported_directory(id)))
            .map_err(|error| AssetError::InvalidPath(error.to_string()))
    }


}
