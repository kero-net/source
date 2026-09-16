use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

use super::{Diagnostic, KnowledgeSetId, Mount, MountId};

pub const PROJECT_DIRECTORY: &str = ".kero";
pub const PROJECT_FILE: &str = "project.toml";
pub const PROJECT_SCHEMA: &str = "kero/project/v1alpha1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectBoundary {
    pub root: PathBuf,
    pub directory: PathBuf,
    pub declaration: PathBuf,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema: String,
    pub id: KnowledgeSetId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mounts: Vec<Mount>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, toml::Value>,
}

impl Project {
    pub fn new(id: KnowledgeSetId) -> Self {
        Self {
            schema: PROJECT_SCHEMA.into(),
            id,
            mounts: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.schema != PROJECT_SCHEMA {
            return Err(ProjectError::Schema(self.schema.clone()));
        }
        let mut mount_ids = BTreeSet::new();
        let mut source_ids = BTreeSet::new();
        let mut locators = BTreeMap::new();
        let mut orders = BTreeSet::new();
        for mount in &self.mounts {
            if !mount_ids.insert(mount.id.clone()) {
                return Err(ProjectError::DuplicateMount(mount.id.clone()));
            }
            if !source_ids.insert(mount.source_id.clone()) {
                return Err(ProjectError::DuplicateSource(mount.source_id.clone()));
            }
            if mount.source.trim().is_empty() {
                return Err(ProjectError::EmptyLocator(mount.id.clone()));
            }
            let locator = normalized_locator(&mount.source);
            if let Some(existing) = locators.insert(locator, mount.id.clone()) {
                return Err(ProjectError::ConflictingLocator {
                    first: existing,
                    second: mount.id.clone(),
                });
            }
            if !orders.insert(mount.display_order) {
                return Err(ProjectError::DuplicateDisplayOrder(mount.display_order));
            }
        }
        for (expected, actual) in (0_u32..).zip(orders) {
            if expected != actual {
                return Err(ProjectError::DisplayOrderGap { expected, actual });
            }
        }
        Ok(())
    }

    pub fn add_mount(&mut self, mut mount: Mount) -> Result<(), ProjectError> {
        if self.mounts.iter().any(|current| current.id == mount.id) {
            return Err(ProjectError::DuplicateMount(mount.id));
        }
        if self
            .mounts
            .iter()
            .any(|current| current.source_id == mount.source_id)
        {
            return Err(ProjectError::DuplicateSource(mount.source_id));
        }
        let locator = normalized_locator(&mount.source);
        if let Some(existing) = self
            .mounts
            .iter()
            .find(|current| normalized_locator(&current.source) == locator)
        {
            return Err(ProjectError::ConflictingLocator {
                first: existing.id.clone(),
                second: mount.id,
            });
        }
        mount.display_order = self.mounts.len() as u32;
        self.mounts.push(mount);
        self.validate()
    }

    pub fn remove_mount(&mut self, id: &MountId) -> Result<Mount, ProjectError> {
        let index = self
            .mounts
            .iter()
            .position(|mount| &mount.id == id)
            .ok_or_else(|| ProjectError::MountNotFound(id.clone()))?;
        let removed = self.mounts.remove(index);
        self.normalize_display_order();
        Ok(removed)
    }

    pub fn set_mount_enabled(&mut self, id: &MountId, enabled: bool) -> Result<(), ProjectError> {
        let mount = self
            .mounts
            .iter_mut()
            .find(|mount| &mount.id == id)
            .ok_or_else(|| ProjectError::MountNotFound(id.clone()))?;
        mount.enabled = enabled;
        Ok(())
    }

    pub fn reorder_mount(&mut self, id: &MountId, new_index: usize) -> Result<(), ProjectError> {
        if new_index >= self.mounts.len() {
            return Err(ProjectError::DisplayIndex {
                index: new_index,
                length: self.mounts.len(),
            });
        }
        let current = self
            .mounts
            .iter()
            .position(|mount| &mount.id == id)
            .ok_or_else(|| ProjectError::MountNotFound(id.clone()))?;
        let mount = self.mounts.remove(current);
        self.mounts.insert(new_index, mount);
        self.normalize_display_order();
        Ok(())
    }

    fn normalize_display_order(&mut self) {
        for (index, mount) in self.mounts.iter_mut().enumerate() {
            mount.display_order = index as u32;
        }
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("project.declaration-invalid: {0}")]
    Decode(#[from] toml::de::Error),
    #[error("project.declaration-encode: {0}")]
    Encode(#[from] toml::ser::Error),
    #[error("project.not-found: no {PROJECT_DIRECTORY} boundary from {0}")]
    NotFound(PathBuf),
    #[error("project.boundary-symlink: {0}")]
    BoundarySymlink(PathBuf),
    #[error("project.boundary-escape: {0}")]
    BoundaryEscape(PathBuf),
    #[error("project.schema-unsupported: {0}")]
    Schema(String),
    #[error("project.mount-duplicate: {0}")]
    DuplicateMount(MountId),
    #[error("project.source-duplicate: {0}")]
    DuplicateSource(super::SourceId),
    #[error("project.mount-locator-empty: {0}")]
    EmptyLocator(MountId),
    #[error("project.mount-locator-conflict: {first} and {second}")]
    ConflictingLocator { first: MountId, second: MountId },
    #[error("project.display-order-duplicate: {0}")]
    DuplicateDisplayOrder(u32),
    #[error("project.display-order-gap: expected {expected}, found {actual}")]
    DisplayOrderGap { expected: u32, actual: u32 },
    #[error("project.mount-not-found: {0}")]
    MountNotFound(MountId),
    #[error("project.display-index-invalid: {index} for {length} mounts")]
    DisplayIndex { index: usize, length: usize },
    #[error("project.exists: {0}")]
    Exists(PathBuf),
}

impl ProjectError {
    pub fn diagnostic(&self) -> Diagnostic {
        let code = match self {
            Self::Io(_) => "project.io",
            Self::Decode(_) => "project.declaration-invalid",
            Self::Encode(_) => "project.declaration-encode",
            Self::NotFound(_) => "project.not-found",
            Self::BoundarySymlink(_) => "project.boundary-symlink",
            Self::BoundaryEscape(_) => "project.boundary-escape",
            Self::Schema(_) => "project.schema-unsupported",
            Self::DuplicateMount(_) => "project.mount-duplicate",
            Self::DuplicateSource(_) => "project.source-duplicate",
            Self::EmptyLocator(_) => "project.mount-locator-empty",
            Self::ConflictingLocator { .. } => "project.mount-locator-conflict",
            Self::DuplicateDisplayOrder(_) => "project.display-order-duplicate",
            Self::DisplayOrderGap { .. } => "project.display-order-gap",
            Self::MountNotFound(_) => "project.mount-not-found",
            Self::DisplayIndex { .. } => "project.display-index-invalid",
            Self::Exists(_) => "project.exists",
        };
        Diagnostic::error(code, self.to_string())
    }
}

/// Create the minimum reviewable declaration. Discovery suggestions are never added.
pub fn initialize(root: &Path, id: KnowledgeSetId) -> Result<ProjectBoundary, ProjectError> {
    let directory = root.join(PROJECT_DIRECTORY);
    if directory.exists() {
        return Err(ProjectError::Exists(directory));
    }
    fs::create_dir(&directory)?;
    let boundary = ProjectBoundary {
        root: root.canonicalize()?,
        declaration: directory.join(PROJECT_FILE),
        directory,
    };
    let project = Project::new(id);
    let output = toml::to_string_pretty(&project)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&boundary.declaration)?
        .write_all(output.as_bytes())?;
    Ok(boundary)
}

/// Conservative, deterministic candidates requiring explicit caller approval.
pub fn source_candidates(root: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let mut values = Vec::new();
    for name in ["README.md", "README.markdown", "README"] {
        if root.join(name).is_file() {
            values.push(PathBuf::from(name));
        }
    }
    let docs = root.join("docs");
    if docs.is_dir() {
        for entry in fs::read_dir(docs)? {
            let path = entry?.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|v| v.eq_ignore_ascii_case("md"))
            {
                values.push(path.strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    values.sort();
    values.dedup();
    Ok(values)
}

/// Hash semantic declaration data independently from TOML whitespace and key order.
pub fn declaration_digest(project: &Project) -> Result<String, ProjectError> {
    project.validate()?;
    let bytes = serde_json::to_vec(project).expect("project is serializable");
    let mut hash = Sha256::new();
    hash.update(b"kero/project-declaration-hash/v1alpha1\0");
    hash.update(bytes);
    Ok(format!("sha256:{}", hex::encode(hash.finalize())))
}

pub fn discover(start: &Path) -> Result<ProjectBoundary, ProjectError> {
    let original = start.to_path_buf();
    let absolute = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()?.join(start)
    };
    let metadata = fs::metadata(&absolute)?;
    let mut cursor = if metadata.is_file() {
        absolute
            .parent()
            .ok_or_else(|| ProjectError::NotFound(original.clone()))?
            .to_path_buf()
    } else {
        absolute
    };

    loop {
        let directory = cursor.join(PROJECT_DIRECTORY);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(ProjectError::BoundarySymlink(directory));
            }
            Ok(metadata) if metadata.is_dir() => {
                let root = cursor.canonicalize()?;
                let canonical_directory = directory.canonicalize()?;
                if canonical_directory.parent() != Some(root.as_path()) {
                    return Err(ProjectError::BoundaryEscape(directory));
                }
                return Ok(ProjectBoundary {
                    declaration: canonical_directory.join(PROJECT_FILE),
                    directory: canonical_directory,
                    root,
                });
            }
            Ok(_) => return Err(ProjectError::BoundaryEscape(directory)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        if !cursor.pop() {
            return Err(ProjectError::NotFound(original));
        }
    }
}

pub fn load(boundary: &ProjectBoundary) -> Result<Project, ProjectError> {
    let input = fs::read_to_string(&boundary.declaration)?;
    let project: Project = toml::from_str(&input)?;
    project.validate()?;
    Ok(project)
}

pub fn save(boundary: &ProjectBoundary, project: &Project) -> Result<(), ProjectError> {
    project.validate()?;
    let mut normalized = project.clone();
    normalized.mounts.sort_by_key(|mount| mount.display_order);
    let output = toml::to_string_pretty(&normalized)?;
    let temporary = boundary.directory.join(".project.toml.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = (|| {
        file.write_all(output.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, &boundary.declaration)
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(())
}

fn normalized_locator(locator: &str) -> String {
    let path = Path::new(locator);
    let mut parts: Vec<String> = Vec::new();
    let mut prefix = String::new();
    for component in path.components() {
        match component {
            Component::Prefix(value) => prefix = value.as_os_str().to_string_lossy().into_owned(),
            Component::RootDir => prefix.push('/'),
            Component::CurDir => {}
            Component::ParentDir if parts.last().is_some_and(|part| part != "..") => {
                parts.pop();
            }
            Component::ParentDir => parts.push("..".into()),
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
        }
    }
    format!("{}{}", prefix, parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{MountKind, SourceId};

    fn mount(id: &str, source_id: &str, source: &str, order: u32) -> Mount {
        Mount::new(
            MountId::new(id).unwrap(),
            SourceId::new(source_id).unwrap(),
            source,
            MountKind::LocalFile,
            order,
        )
    }

    #[test]
    fn knowledge_project_operations_share_one_reviewable_model() {
        let mut project = Project::new(KnowledgeSetId::new("example.project").unwrap());
        project
            .add_mount(mount("docs", "source.docs", "docs.md", 9))
            .unwrap();
        project
            .add_mount(mount("readme", "source.readme", "README.md", 9))
            .unwrap();
        let docs = MountId::new("docs").unwrap();
        project.reorder_mount(&docs, 1).unwrap();
        project.set_mount_enabled(&docs, false).unwrap();
        assert_eq!(project.mounts[1].id, docs);
        assert!(!project.mounts[1].enabled);
        assert_eq!(project.mounts[1].display_order, 1);
        project.remove_mount(&docs).unwrap();
        assert_eq!(project.mounts[0].display_order, 0);
        project.validate().unwrap();
    }

    #[test]
    fn knowledge_project_rejects_conflicting_normalized_locators() {
        let mut project = Project::new(KnowledgeSetId::new("example.project").unwrap());
        project
            .add_mount(mount("one", "source.one", "docs/guide.md", 0))
            .unwrap();
        let error = project
            .add_mount(mount("two", "source.two", "docs/./guide.md", 1))
            .unwrap_err();
        assert_eq!(error.diagnostic().code, "project.mount-locator-conflict");
    }

    #[test]
    fn knowledge_project_discovers_nearest_nested_boundary() {
        let temp = tempfile::tempdir().unwrap();
        let outer = temp.path().join("outer");
        let inner = outer.join("nested");
        let leaf = inner.join("src/module");
        fs::create_dir_all(outer.join(PROJECT_DIRECTORY)).unwrap();
        fs::create_dir_all(inner.join(PROJECT_DIRECTORY)).unwrap();
        fs::create_dir_all(&leaf).unwrap();
        let found = discover(&leaf).unwrap();
        assert_eq!(found.root, inner.canonicalize().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn knowledge_project_rejects_symlinked_boundary() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let elsewhere = temp.path().join("elsewhere");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();
        symlink(&elsewhere, project.join(PROJECT_DIRECTORY)).unwrap();
        assert!(matches!(
            discover(&project),
            Err(ProjectError::BoundarySymlink(_))
        ));
    }
}
