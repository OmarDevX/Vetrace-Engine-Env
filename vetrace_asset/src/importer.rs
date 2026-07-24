use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vetrace_project::{ProjectPath, ProjectPaths};

use crate::{AssetError, AssetId, AssetKind, AssetResult};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImporterStamp {
    pub id: String,
    pub version: u32,
}

pub struct ImportContext<'a> {
    pub id: AssetId,
    pub source: &'a ProjectPath,
    pub source_path: &'a Path,
    pub project_paths: &'a ProjectPaths,
    pub output_directory: &'a Path,
}

#[derive(Clone, Debug, Default)]
pub struct ImportOutput {
    pub outputs: Vec<PathBuf>,
    pub dependencies: Vec<ProjectPath>,
    pub metadata: BTreeMap<String, String>,
}

pub trait AssetImporter: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> u32;
    fn kind(&self) -> AssetKind;
    fn extensions(&self) -> &[String];
    fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput>;
}

#[derive(Default)]
pub struct ImporterRegistry {
    importers: BTreeMap<String, Arc<dyn AssetImporter>>,
    extensions: BTreeMap<String, String>,
}

impl ImporterRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register<I: AssetImporter + 'static>(&mut self, importer: I) {
        self.register_arc(Arc::new(importer));
    }

    pub fn register_arc(&mut self, importer: Arc<dyn AssetImporter>) {
        let id = importer.id().to_string();

        // Replacing an importer must also replace its extension ownership. Otherwise,
        // extensions removed by a newer importer version would keep pointing at the
        // old registration ID indefinitely.
        self.extensions.retain(|_, registered_id| registered_id != &id);
        for extension in importer.extensions() {
            self.extensions.insert(normalize_extension(extension), id.clone());
        }
        self.importers.insert(id, importer);
    }

    pub fn importer_for_path(&self, path: &Path) -> Option<Arc<dyn AssetImporter>> {
        let extension = path.extension()?.to_str()?;
        let id = self.extensions.get(&normalize_extension(extension))?;
        self.importers.get(id).cloned()
    }

    pub fn importer(&self, id: &str) -> Option<Arc<dyn AssetImporter>> {
        self.importers.get(id).cloned()
    }

    pub fn len(&self) -> usize { self.importers.len() }
    pub fn is_empty(&self) -> bool { self.importers.is_empty() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyScanner {
    None,
    ProjectPathsInText,
    JsonProjectPaths,
    GltfUris,
}

pub struct GenericCopyImporter {
    id: String,
    version: u32,
    kind: AssetKind,
    extensions: Vec<String>,
    scanner: DependencyScanner,
}

impl GenericCopyImporter {
    pub fn new<I, S>(
        id: impl Into<String>,
        version: u32,
        kind: AssetKind,
        extensions: I,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            id: id.into(),
            version,
            kind,
            extensions: extensions.into_iter().map(Into::into).collect(),
            scanner: DependencyScanner::None,
        }
    }

    pub fn dependency_scanner(mut self, scanner: DependencyScanner) -> Self {
        self.scanner = scanner;
        self
    }
}

impl AssetImporter for GenericCopyImporter {
    fn id(&self) -> &str { &self.id }
    fn version(&self) -> u32 { self.version }
    fn kind(&self) -> AssetKind { self.kind.clone() }
    fn extensions(&self) -> &[String] { &self.extensions }

    fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput> {
        fs::create_dir_all(context.output_directory).map_err(|error| {
            AssetError::io("create imported asset directory", context.output_directory, error)
        })?;
        let file_name = context.source_path.file_name().ok_or_else(|| {
            AssetError::Importer(format!("asset '{}' has no file name", context.source))
        })?;
        let output = context.output_directory.join(file_name);
        fs::copy(context.source_path, &output)
            .map_err(|error| AssetError::io("copy imported asset", &output, error))?;
        let dependencies = scan_dependencies(self.scanner, context)?;
        let mut metadata = BTreeMap::new();
        metadata.insert("source_file".into(), context.source.to_string());
        metadata.insert("copy_mode".into(), "source_preserving".into());
        Ok(ImportOutput { outputs: vec![output], dependencies, metadata })
    }
}

pub fn register_builtin_importers(registry: &mut ImporterRegistry) {
    registry.register(GenericCopyImporter::new(
        "vetrace.scene", 1, AssetKind::Scene, ["vscene"],
    ).dependency_scanner(DependencyScanner::JsonProjectPaths));
    registry.register(GenericCopyImporter::new(
        "vetrace.lua", 1, AssetKind::Script, ["lua"],
    ).dependency_scanner(DependencyScanner::ProjectPathsInText));
    registry.register(crate::builtin_importers::ModelImporter::default());
    registry.register(crate::builtin_importers::TextureImporter::default());
    registry.register(GenericCopyImporter::new(
        "vetrace.texture.container", 2, AssetKind::Texture, ["hdr", "exr", "dds", "ktx2"],
    ));
    registry.register(crate::builtin_importers::WaveAudioImporter::default());
    registry.register(GenericCopyImporter::new(
        "vetrace.audio.compressed", 2, AssetKind::Audio, ["ogg", "mp3", "flac"],
    ));
    registry.register(GenericCopyImporter::new(
        "vetrace.font", 1, AssetKind::Font, ["ttf", "otf"],
    ));
    registry.register(crate::builtin_importers::ShaderImporter::default());
    registry.register(GenericCopyImporter::new(
        "vetrace.material", 1, AssetKind::Material, ["vmat"],
    ).dependency_scanner(DependencyScanner::ProjectPathsInText));
    registry.register(GenericCopyImporter::new(
        "vetrace.data", 1, AssetKind::Data, ["json", "toml", "ron", "csv", "txt"],
    ).dependency_scanner(DependencyScanner::ProjectPathsInText));
}

fn normalize_extension(extension: &str) -> String {
    extension.trim_start_matches('.').to_ascii_lowercase()
}

pub(crate) fn scan_dependencies(scanner: DependencyScanner, context: &ImportContext<'_>) -> AssetResult<Vec<ProjectPath>> {
    match scanner {
        DependencyScanner::None => Ok(Vec::new()),
        DependencyScanner::ProjectPathsInText => {
            let text = fs::read_to_string(context.source_path)
                .map_err(|error| AssetError::io("read asset dependencies", context.source_path, error))?;
            Ok(scan_project_paths_in_text(&text))
        }
        DependencyScanner::JsonProjectPaths => {
            let bytes = fs::read(context.source_path)
                .map_err(|error| AssetError::io("read JSON asset", context.source_path, error))?;
            let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
                AssetError::Importer(format!("failed to parse '{}': {error}", context.source))
            })?;
            let mut paths = BTreeSet::new();
            collect_json_project_paths(&value, &mut paths);
            Ok(paths.into_iter().collect())
        }
        DependencyScanner::GltfUris => scan_gltf_dependencies(context),
    }
}

fn scan_project_paths_in_text(text: &str) -> Vec<ProjectPath> {
    let mut paths = BTreeSet::new();
    for token in text.split(|character: char| {
        character.is_whitespace()
            || matches!(character, '"' | '\'' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';')
    }) {
        let candidate = token.trim_matches(|character: char| matches!(character, ':' | '='));
        if candidate.starts_with("assets/") {
            if let Ok(path) = ProjectPath::new(candidate) { paths.insert(path); }
        }
    }
    paths.into_iter().collect()
}

fn collect_json_project_paths(value: &Value, output: &mut BTreeSet<ProjectPath>) {
    match value {
        Value::String(value) if value.starts_with("assets/") => {
            if let Ok(path) = ProjectPath::new(value) { output.insert(path); }
        }
        Value::Array(values) => {
            for value in values { collect_json_project_paths(value, output); }
        }
        Value::Object(values) => {
            for value in values.values() { collect_json_project_paths(value, output); }
        }
        _ => {}
    }
}

fn scan_gltf_dependencies(context: &ImportContext<'_>) -> AssetResult<Vec<ProjectPath>> {
    let bytes = fs::read(context.source_path)
        .map_err(|error| AssetError::io("read glTF asset", context.source_path, error))?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        AssetError::Importer(format!("failed to parse glTF '{}': {error}", context.source))
    })?;
    let mut uris = BTreeSet::new();
    for collection in ["buffers", "images"] {
        if let Some(values) = value.get(collection).and_then(Value::as_array) {
            for value in values {
                let Some(uri) = value.get("uri").and_then(Value::as_str) else { continue; };
                if uri.starts_with("data:") || uri.contains("://") || Path::new(uri).is_absolute() {
                    continue;
                }
                let parent = context.source.as_path().parent().unwrap_or(Path::new("assets"));
                let joined = normalize_relative_path(parent.join(uri));
                if let Ok(path) = ProjectPath::new(joined.to_string_lossy().replace('\\', "/")) {
                    uris.insert(path);
                }
            }
        }
    }
    Ok(uris.into_iter().collect())
}

fn normalize_relative_path(path: PathBuf) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::CurDir => {}
            Component::ParentDir => { output.pop(); }
            Component::Normal(value) => output.push(value),
            _ => {}
        }
    }
    output
}
