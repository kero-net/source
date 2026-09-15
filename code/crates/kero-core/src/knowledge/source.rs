use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

use super::{
    ContentDigest, Diagnostic, Mount, MountId, MountKind, SourceId, SourceRegionId,
    SourceRevisionId,
};

pub const DIGEST_ALGORITHM: &str = "sha256-v1";
pub const REVISION_ALGORITHM: &str = "source-revision/sha256-v1";
pub const REGION_ALGORITHM: &str = "source-region/sha256-v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    Disabled,
    Unavailable,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRevision {
    pub id: SourceRevisionId,
    pub source_id: SourceId,
    pub content_digest: ContentDigest,
    pub media_type: String,
    pub byte_length: u64,
    pub acquisition: BTreeMap<String, String>,
}

impl SourceRevision {
    pub fn from_bytes(
        source_id: SourceId,
        media_type: impl Into<String>,
        bytes: &[u8],
        acquisition: BTreeMap<String, String>,
    ) -> Self {
        let media_type = media_type.into();
        let content_digest = ContentDigest::new(digest(bytes)).expect("digest is valid");
        let mut identity = Sha256::new();
        hash_part(&mut identity, REVISION_ALGORITHM.as_bytes());
        hash_part(&mut identity, source_id.as_str().as_bytes());
        hash_part(&mut identity, content_digest.as_str().as_bytes());
        hash_part(&mut identity, media_type.as_bytes());
        for (key, value) in &acquisition {
            hash_part(&mut identity, key.as_bytes());
            hash_part(&mut identity, value.as_bytes());
        }
        let id = SourceRevisionId::new(format!("sha256:{}", hex::encode(identity.finalize())))
            .expect("digest is valid");
        Self {
            id,
            source_id,
            content_digest,
            media_type,
            byte_length: bytes.len() as u64,
            acquisition,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: SourceId,
    pub mount_id: MountId,
    pub kind: MountKind,
    pub logical_locator: String,
    pub media_type: String,
    pub display_order: u32,
    pub availability: Availability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<SourceRevision>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub acquisition: BTreeMap<String, String>,
}

impl Source {
    fn declared(mount: &Mount, availability: Availability) -> Self {
        Self {
            id: mount.source_id.clone(),
            mount_id: mount.id.clone(),
            kind: mount.kind.clone(),
            logical_locator: mount.source.clone(),
            media_type: mount
                .media_type
                .clone()
                .unwrap_or_else(|| default_media_type(&mount.kind).into()),
            display_order: mount.display_order,
            availability,
            revision: None,
            acquisition: mount.acquisition.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRegion {
    pub id: SourceRegionId,
    pub source_id: SourceId,
    pub revision_id: SourceRevisionId,
    pub start: u64,
    pub end: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<String>,
}

impl SourceRegion {
    pub fn utf8(
        revision: &SourceRevision,
        text: &str,
        start: usize,
        end: usize,
        discriminator: Option<String>,
    ) -> Result<Self, SourceError> {
        if start > end || end > text.len() {
            return Err(SourceError::RegionBounds {
                start,
                end,
                length: text.len(),
            });
        }
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return Err(SourceError::RegionUtf8Boundary { start, end });
        }
        let (start_line, start_column) = line_column(text, start);
        let (end_line, end_column) = line_column(text, end);
        let mut identity = Sha256::new();
        hash_part(&mut identity, REGION_ALGORITHM.as_bytes());
        hash_part(&mut identity, revision.id.as_str().as_bytes());
        hash_part(&mut identity, &(start as u64).to_be_bytes());
        hash_part(&mut identity, &(end as u64).to_be_bytes());
        if let Some(value) = &discriminator {
            hash_part(&mut identity, value.as_bytes());
        }
        let id = SourceRegionId::new(format!(
            "region:sha256:{}",
            hex::encode(identity.finalize())
        ))
        .expect("digest is valid");
        Ok(Self {
            id,
            source_id: revision.source_id.clone(),
            revision_id: revision.id.clone(),
            start: start as u64,
            end: end as u64,
            start_line: Some(start_line),
            start_column: Some(start_column),
            end_line: Some(end_line),
            end_column: Some(end_column),
            discriminator,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpaqueSource {
    pub source: Source,
    pub reason: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum SourceObservation {
    Current {
        source: Source,
    },
    Disabled {
        source: Source,
    },
    Unavailable {
        source: Source,
        diagnostic: Diagnostic,
    },
    Unsupported {
        opaque: OpaqueSource,
    },
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("source.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("source.locator-symlink: {0}")]
    LocatorSymlink(PathBuf),
    #[error("source.locator-not-file: {0}")]
    NotFile(PathBuf),
    #[error("source.digest-mismatch: expected {expected}, found {actual}")]
    DigestMismatch {
        expected: ContentDigest,
        actual: ContentDigest,
    },
    #[error("source.region-bounds: {start}..{end} exceeds {length} bytes")]
    RegionBounds {
        start: usize,
        end: usize,
        length: usize,
    },
    #[error("source.region-utf8-boundary: {start}..{end}")]
    RegionUtf8Boundary { start: usize, end: usize },
}

impl SourceError {
    pub fn diagnostic(&self, source_id: Option<SourceId>) -> Diagnostic {
        let code = match self {
            Self::Io(_) => "source.io",
            Self::LocatorSymlink(_) => "source.locator-symlink",
            Self::NotFile(_) => "source.locator-not-file",
            Self::DigestMismatch { .. } => "source.digest-mismatch",
            Self::RegionBounds { .. } => "source.region-bounds",
            Self::RegionUtf8Boundary { .. } => "source.region-utf8-boundary",
        };
        let diagnostic = Diagnostic::error(code, self.to_string());
        match source_id {
            Some(source_id) => diagnostic.with_source(source_id),
            None => diagnostic,
        }
    }
}

pub fn observe_mount(project_root: &Path, mount: &Mount) -> Result<SourceObservation, SourceError> {
    if !mount.enabled {
        return Ok(SourceObservation::Disabled {
            source: Source::declared(mount, Availability::Disabled),
        });
    }
    if !mount.kind.is_locally_acquirable() {
        let source = Source::declared(mount, Availability::Unsupported);
        return Ok(SourceObservation::Unsupported {
            opaque: OpaqueSource {
                diagnostics: vec![
                    Diagnostic::error(
                        "source.kind-unsupported",
                        format!("source kind {:?} is not acquired in Phase 2", mount.kind),
                    )
                    .with_source(mount.source_id.clone()),
                ],
                reason: "source kind is preserved but has no Phase 2 acquirer".into(),
                source,
            },
        });
    }

    let path = resolve_without_symlinks(project_root, &mount.source);
    let path = match path {
        Ok(path) => path,
        Err(error) if matches!(&error, SourceError::Io(io) if io.kind() == std::io::ErrorKind::NotFound) =>
        {
            let source = Source::declared(mount, Availability::Unavailable);
            return Ok(SourceObservation::Unavailable {
                diagnostic: Diagnostic::error("source.unavailable", error.to_string())
                    .with_source(mount.source_id.clone()),
                source,
            });
        }
        Err(error) => return Err(error),
    };
    if !path.is_file() {
        return Err(SourceError::NotFile(path));
    }
    let bytes = fs::read(&path)?;
    let mut source = Source::declared(mount, Availability::Available);
    let revision = SourceRevision::from_bytes(
        source.id.clone(),
        source.media_type.clone(),
        &bytes,
        source.acquisition.clone(),
    );
    if let Some(expected) = &mount.expected_digest {
        if expected != &revision.content_digest {
            return Err(SourceError::DigestMismatch {
                expected: expected.clone(),
                actual: revision.content_digest,
            });
        }
    }
    source.revision = Some(revision);
    Ok(SourceObservation::Current { source })
}

fn resolve_without_symlinks(root: &Path, locator: &str) -> Result<PathBuf, SourceError> {
    let target = if Path::new(locator).is_absolute() {
        PathBuf::from(locator)
    } else {
        root.join(locator)
    };
    let absolute = lexical_absolute(&target)?;
    let mut cursor = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => cursor.push(prefix.as_os_str()),
            Component::RootDir => cursor.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                cursor.pop();
            }
            Component::Normal(value) => cursor.push(value),
        }
        if cursor.as_os_str().is_empty() || cursor.parent().is_none() {
            continue;
        }
        let metadata = fs::symlink_metadata(&cursor)?;
        if metadata.file_type().is_symlink() {
            return Err(SourceError::LocatorSymlink(cursor));
        }
    }
    Ok(absolute)
}

fn lexical_absolute(path: &Path) -> Result<PathBuf, std::io::Error> {
    let input = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut output = PathBuf::new();
    for component in input.components() {
        match component {
            Component::Prefix(prefix) => output.push(prefix.as_os_str()),
            Component::RootDir => output.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                output.pop();
            }
            Component::Normal(value) => output.push(value),
        }
    }
    Ok(output)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

fn hash_part(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn default_media_type(kind: &MountKind) -> &'static str {
    match kind {
        MountKind::StructuredJson => "application/json",
        MountKind::LocalFile => "text/markdown; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn line_column(text: &str, byte: usize) -> (u64, u64) {
    let prefix = &text[..byte];
    let line = prefix.bytes().filter(|value| *value == b'\n').count() as u64 + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count(), |(_, tail)| tail.chars().count()) as u64
        + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{MountId, SourceId};

    fn mount(id: &str, source_id: &str, locator: &str) -> Mount {
        Mount::new(
            MountId::new(id).unwrap(),
            SourceId::new(source_id).unwrap(),
            locator,
            MountKind::LocalFile,
            0,
        )
    }

    #[test]
    fn knowledge_source_same_content_retains_distinct_source_identity() {
        let bytes = b"same knowledge";
        let left = SourceRevision::from_bytes(
            SourceId::new("source.left").unwrap(),
            "text/plain",
            bytes,
            BTreeMap::new(),
        );
        let right = SourceRevision::from_bytes(
            SourceId::new("source.right").unwrap(),
            "text/plain",
            bytes,
            BTreeMap::new(),
        );
        assert_eq!(left.content_digest, right.content_digest);
        assert_ne!(left.id, right.id);
    }

    #[test]
    fn knowledge_source_revision_changes_with_bytes_not_locator() {
        let source_id = SourceId::new("source.docs").unwrap();
        let before =
            SourceRevision::from_bytes(source_id.clone(), "text/plain", b"before", BTreeMap::new());
        let same_after_move =
            SourceRevision::from_bytes(source_id.clone(), "text/plain", b"before", BTreeMap::new());
        let changed =
            SourceRevision::from_bytes(source_id, "text/plain", b"after", BTreeMap::new());
        assert_eq!(before.id, same_after_move.id);
        assert_ne!(before.id, changed.id);
    }

    #[test]
    fn knowledge_source_region_validates_utf8_and_bounds() {
        let revision = SourceRevision::from_bytes(
            SourceId::new("source.docs").unwrap(),
            "text/plain",
            "aé\nz".as_bytes(),
            BTreeMap::new(),
        );
        let region = SourceRegion::utf8(&revision, "aé\nz", 1, 3, None).unwrap();
        assert_eq!((region.start_line, region.start_column), (Some(1), Some(2)));
        assert!(matches!(
            SourceRegion::utf8(&revision, "aé\nz", 2, 3, None),
            Err(SourceError::RegionUtf8Boundary { .. })
        ));
        assert!(matches!(
            SourceRegion::utf8(&revision, "aé\nz", 0, 99, None),
            Err(SourceError::RegionBounds { .. })
        ));
    }

    #[test]
    fn knowledge_source_missing_is_unavailable_and_disabled_is_distinct() {
        let temp = tempfile::tempdir().unwrap();
        let missing = mount("missing", "source.missing", "missing.md");
        assert!(matches!(
            observe_mount(temp.path(), &missing).unwrap(),
            SourceObservation::Unavailable { .. }
        ));
        let mut disabled = missing;
        disabled.enabled = false;
        assert!(matches!(
            observe_mount(temp.path(), &disabled).unwrap(),
            SourceObservation::Disabled { .. }
        ));
    }

    #[test]
    fn knowledge_source_expected_digest_mismatch_is_stable() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("docs.md"), "knowledge").unwrap();
        let mut declared = mount("docs", "source.docs", "docs.md");
        declared.expected_digest =
            Some(ContentDigest::new(format!("sha256:{}", "0".repeat(64))).unwrap());
        let error = observe_mount(temp.path(), &declared).unwrap_err();
        assert_eq!(
            error.diagnostic(Some(declared.source_id)).code,
            "source.digest-mismatch"
        );
    }

    #[cfg(unix)]
    #[test]
    fn knowledge_source_rejects_symlinked_locator_component() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("real")).unwrap();
        fs::write(temp.path().join("real/docs.md"), "knowledge").unwrap();
        symlink(temp.path().join("real"), temp.path().join("linked")).unwrap();
        let declared = mount("docs", "source.docs", "linked/docs.md");
        assert!(matches!(
            observe_mount(temp.path(), &declared),
            Err(SourceError::LocatorSymlink(_))
        ));
    }
}
