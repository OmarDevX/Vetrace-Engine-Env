use std::path::{Path, PathBuf};

use crate::{
    three_way_merge, ExternalChange, ExternalChangeKind, ExternalChangeResolution, LanguageError,
    LanguageRegistry, ScriptDocument,
};

#[derive(Default)]
pub struct ScriptWorkspace {
    registry: LanguageRegistry,
    documents: Vec<ScriptDocument>,
    active: Option<usize>,
}

impl ScriptWorkspace {
    pub fn new(registry: LanguageRegistry) -> Self {
        Self { registry, documents: Vec::new(), active: None }
    }

    pub fn registry(&self) -> &LanguageRegistry { &self.registry }
    pub fn documents(&self) -> &[ScriptDocument] { &self.documents }
    pub fn documents_mut(&mut self) -> &mut [ScriptDocument] { &mut self.documents }
    pub fn active_index(&self) -> Option<usize> { self.active }
    pub fn active(&self) -> Option<&ScriptDocument> { self.active.and_then(|index| self.documents.get(index)) }
    pub fn active_mut(&mut self) -> Option<&mut ScriptDocument> {
        self.active.and_then(|index| self.documents.get_mut(index))
    }

    pub fn set_active(&mut self, index: usize) -> bool {
        if index < self.documents.len() {
            self.active = Some(index);
            true
        } else {
            false
        }
    }

    pub fn open(&mut self, path: impl AsRef<Path>) -> Result<usize, LanguageError> {
        let path = path.as_ref();
        if let Some(index) = self.documents.iter().position(|document| document.path == path) {
            self.active = Some(index);
            return Ok(index);
        }
        let service = self.registry.for_path(path).ok_or_else(|| {
            LanguageError(format!("no script language service registered for '{}'", path.display()))
        })?;
        let text = std::fs::read_to_string(path)
            .map_err(|error| LanguageError(format!("failed to open '{}': {error}", path.display())))?;
        let mut document = ScriptDocument::from_text(path, service.language_id(), text);
        document.update_analysis(service.as_ref());
        self.documents.push(document);
        let index = self.documents.len() - 1;
        self.active = Some(index);
        Ok(index)
    }

    pub fn close(&mut self, index: usize, discard: bool) -> Result<(), LanguageError> {
        let Some(document) = self.documents.get(index) else {
            return Err(LanguageError("script tab no longer exists".into()));
        };
        if document.is_dirty() && !discard {
            return Err(LanguageError(format!("'{}' has unsaved changes", document.path.display())));
        }
        self.documents.remove(index);
        self.active = match self.documents.len() {
            0 => None,
            len => Some(index.min(len - 1)),
        };
        Ok(())
    }

    pub fn save(&mut self, index: usize) -> Result<PathBuf, LanguageError> {
        let document = self.documents.get_mut(index)
            .ok_or_else(|| LanguageError("script tab no longer exists".into()))?;
        std::fs::write(&document.path, &document.text)
            .map_err(|error| LanguageError(format!("failed to save '{}': {error}", document.path.display())))?;
        document.mark_saved();
        Ok(document.path.clone())
    }

    pub fn save_active(&mut self) -> Result<PathBuf, LanguageError> {
        let index = self.active.ok_or_else(|| LanguageError("no active script".into()))?;
        self.save(index)
    }

    pub fn rename_document(
        &mut self,
        index: usize,
        new_path: impl AsRef<Path>,
    ) -> Result<PathBuf, LanguageError> {
        let new_path = new_path.as_ref();
        let document = self.documents.get(index)
            .ok_or_else(|| LanguageError("script tab no longer exists".into()))?;
        if self.documents.iter().enumerate().any(|(other, candidate)| other != index && candidate.path == new_path) {
            return Err(LanguageError(format!("'{}' is already open", new_path.display())));
        }
        let service = self.registry.for_path(new_path)
            .ok_or_else(|| LanguageError(format!("no script language service registered for '{}'", new_path.display())))?;
        if service.language_id() != document.language_id {
            return Err(LanguageError("renaming a script cannot change its language".into()));
        }
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                LanguageError(format!("failed to create '{}': {error}", parent.display()))
            })?;
        }
        std::fs::rename(&document.path, new_path).map_err(|error| {
            LanguageError(format!("failed to rename '{}' to '{}': {error}", document.path.display(), new_path.display()))
        })?;
        self.documents[index].set_path(new_path);
        Ok(new_path.to_path_buf())
    }

    pub fn delete_document(&mut self, index: usize, discard: bool) -> Result<PathBuf, LanguageError> {
        let document = self.documents.get(index)
            .ok_or_else(|| LanguageError("script tab no longer exists".into()))?;
        if document.is_dirty() && !discard {
            return Err(LanguageError(format!("'{}' has unsaved changes", document.path.display())));
        }
        let path = document.path.clone();
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(LanguageError(format!("failed to delete '{}': {error}", path.display()))),
        }
        self.close(index, true)?;
        Ok(path)
    }

    pub fn poll_external_changes(&self) -> Vec<ExternalChange> {
        self.documents.iter().enumerate().filter_map(|(document_index, document)| {
            match std::fs::read_to_string(&document.path) {
                Ok(disk_text) if disk_text != document.saved_text => Some(ExternalChange {
                    document_index,
                    path: document.path.clone(),
                    kind: ExternalChangeKind::Modified,
                    disk_text: Some(disk_text),
                    has_local_changes: document.is_dirty(),
                }),
                Ok(_) => None,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(ExternalChange {
                    document_index,
                    path: document.path.clone(),
                    kind: ExternalChangeKind::Deleted,
                    disk_text: None,
                    has_local_changes: document.is_dirty(),
                }),
                Err(_) => None,
            }
        }).collect()
    }

    pub fn resolve_external_change(
        &mut self,
        change: &ExternalChange,
        resolution: ExternalChangeResolution,
    ) -> Result<(), LanguageError> {
        let document = self.documents.get_mut(change.document_index)
            .ok_or_else(|| LanguageError("script tab no longer exists".into()))?;
        if document.path != change.path {
            return Err(LanguageError("script changed while resolving an external edit".into()));
        }
        match (change.kind, resolution) {
            (ExternalChangeKind::Deleted, ExternalChangeResolution::Reload) => {
                return Err(LanguageError("the script was deleted outside Studio".into()));
            }
            (ExternalChangeKind::Deleted, ExternalChangeResolution::KeepLocal)
            | (ExternalChangeKind::Deleted, ExternalChangeResolution::Merge) => {
                document.saved_text.clear();
            }
            (ExternalChangeKind::Modified, ExternalChangeResolution::Reload) => {
                document.replace_from_disk(change.disk_text.clone().unwrap_or_default());
            }
            (ExternalChangeKind::Modified, ExternalChangeResolution::KeepLocal) => {
                document.saved_text = change.disk_text.clone().unwrap_or_default();
                document.revision = document.revision.saturating_add(1);
            }
            (ExternalChangeKind::Modified, ExternalChangeResolution::Merge) => {
                let disk = change.disk_text.as_deref().unwrap_or_default();
                let merged = three_way_merge(&document.saved_text, &document.text, disk);
                document.saved_text = disk.to_owned();
                document.set_text(merged);
            }
        }
        Ok(())
    }

    pub fn analyze(&mut self, index: usize) -> Result<(), LanguageError> {
        let language_id = self.documents.get(index)
            .ok_or_else(|| LanguageError("script tab no longer exists".into()))?
            .language_id.clone();
        let service = self.registry.get(&language_id)
            .ok_or_else(|| LanguageError(format!("language service '{language_id}' is unavailable")))?;
        self.documents[index].update_analysis(service.as_ref());
        Ok(())
    }

    pub fn analyze_active(&mut self) -> Result<(), LanguageError> {
        let index = self.active.ok_or_else(|| LanguageError("no active script".into()))?;
        self.analyze(index)
    }
}
