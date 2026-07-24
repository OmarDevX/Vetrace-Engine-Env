//! Toolchain-free project packaging and export for Vetrace.
//!
//! `vetrace_build` never invokes Cargo. It validates a project, creates a
//! versioned `.vpak`, and copies a caller-supplied prebuilt player template.

mod config;
mod distribution;
mod error;
#[cfg(feature = "export")]
mod export;
mod package;
mod template;
mod template_manager;

pub use config::{
    CompressionMode, ExportConfig, ExportPreset, ExportTarget, EXPORT_CONFIG_FILE,
    EXPORT_CONFIG_FORMAT_VERSION,
};
pub use error::{BuildError, BuildResult};
pub use distribution::{
    package_linux_appimage, package_macos_app, package_portable_zip, package_windows_nsis,
    DistributionArtifact,
};
#[cfg(feature = "export")]
pub use export::{
    build_project, BuildAssetPreflight, BuildReport, BuildRequest, BUILD_REPORT_FILE,
};
pub use package::{
    create_vpak, inspect_vpak, mount_vpak, PackageEntry, PackageManifest, PackageMount,
    PackageOptions, VPAK_FORMAT_VERSION, VPAK_MANIFEST_FILE,
};
pub use template::{
    default_executable_name, find_player_template, load_player_template_metadata,
    player_template_metadata_path, sanitize_executable_name, validate_player_template,
    write_player_template_metadata, PlayerTemplateMetadata, PlayerTemplateTarget,
    PLAYER_TEMPLATE_METADATA_FORMAT_VERSION, PLAYER_TEMPLATE_METADATA_SUFFIX,
};

pub use template_manager::{
    create_template_bundle, default_template_root, InstalledPlayerTemplate,
    PlayerTemplateCatalog, PlayerTemplateCatalogEntry, PlayerTemplateManager,
    TEMPLATE_BUNDLE_MANIFEST, TEMPLATE_CATALOG_FORMAT_VERSION, TEMPLATE_INDEX_FORMAT_VERSION,
};
