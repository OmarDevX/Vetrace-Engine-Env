//! Generic project asset discovery, import, cache, and diagnostics.
//!
//! The crate deliberately has no rendering, ECS, editor, or runtime dependency.
//! Importers are registered through [`ImporterRegistry`], so custom plugins can
//! add asset types without changing Vetrace Studio or this crate.

mod database;
mod builtin_importers;
mod error;
mod filesystem;
mod importer;
mod manager;
mod record;
mod scan;
mod watcher;

pub use database::{AssetDatabase, ASSET_DATABASE_FORMAT_VERSION, ASSET_DATABASE_PATH};
pub use error::{AssetError, AssetResult};
pub use importer::{
    register_builtin_importers, AssetImporter, DependencyScanner, GenericCopyImporter,
    ImportContext, ImportOutput, ImporterRegistry, ImporterStamp,
};
pub use manager::{AssetManager, AssetRefreshReport, CacheStats, ImportedExternalFile};
pub use record::{
    AssetDependency, AssetDiagnostic, AssetDiagnosticSeverity, AssetId, AssetKind, AssetRecord,
    AssetStatus,
};
pub use watcher::{AssetChangeBatch, AssetWatcher};
