//! Deterministic native capture of explicitly selected knowledge input.

use crate::host::native_repository::{RUNTIME_DIRECTORY, RepositoryBoundary};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

const INPUT_DIRECTORY: &str = "input";
const FORMAT_VERSION: u32 = 1;

/// A durable summary of one locally captured knowledge input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputSnapshot {
    /// The content-addressed identifier of the captured bytes.
    pub id: String,
    /// The snapshot format used to calculate the identifier.
    pub format_version: u32,
    /// The number of regular files captured in the snapshot.
    pub files: u64,
}

/// Explicit input capture failures.
#[derive(Debug, Error)]
pub enum InputError {
    #[error("knowledge-input.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("knowledge-input.source-invalid: {0}")]
    SourceInvalid(String),
    #[error("knowledge-input.id-invalid: {0}")]
    InvalidId(String),
    #[error("knowledge-input.not-found: {0}")]
    NotFound(String),
    #[error("knowledge-input.manifest: {0}")]
    Manifest(String),
}

#[derive(Clone, Debug)]
struct SourceEntry {
    relative: PathBuf,
    source: PathBuf,
    directory: bool,
}

/// Captures one explicit file or directory into repository-local input storage.
///
/// The identifier is computed from the ordered tree shape and exact bytes
/// copied into the snapshot. Existing identical snapshots are left unchanged.
pub fn capture(boundary: &RepositoryBoundary, source: &Path) -> Result<InputSnapshot, InputError> {
    let source = source
        .canonicalize()
        .map_err(|error| InputError::SourceInvalid(format!("{}: {error}", source.display())))?;
    let data = boundary.data.canonicalize()?;
    let mounts = boundary.mounts.canonicalize()?;
    if source.starts_with(&data) || source.starts_with(&mounts) {
        return Err(InputError::SourceInvalid(
            "the selected source is already KERO local or mounted data".into(),
        ));
    }

    let entries = collect_entries(&source)?;
    let runtime = boundary.directory.join(RUNTIME_DIRECTORY).join("input");
    fs::create_dir_all(&runtime)?;
    let staging = create_staging_directory(&runtime)?;
    let content = staging.join("content");
    fs::create_dir(&content)?;

    let result = write_snapshot(&entries, &content);
    let (id, files) = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    let snapshot = InputSnapshot {
        id: id.clone(),
        format_version: FORMAT_VERSION,
        files,
    };
    fs::write(
        staging.join("manifest.json"),
        serde_json::to_vec_pretty(&snapshot)
            .map_err(|error| InputError::Manifest(error.to_string()))?,
    )?;

    let inputs = data.join(INPUT_DIRECTORY);
    fs::create_dir_all(&inputs)?;
    let destination = inputs.join(&id);
    if destination.exists() {
        fs::remove_dir_all(&staging)?;
        return read_snapshot(&destination);
    }
    fs::rename(&staging, &destination)?;
    read_snapshot(&destination)
}

fn create_staging_directory(runtime: &Path) -> Result<PathBuf, InputError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| InputError::SourceInvalid(format!("clock is unavailable: {error}")))?
        .as_nanos();
    for attempt in 0..32_u32 {
        let path = runtime.join(format!("capture-{}-{nonce}-{attempt}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(InputError::SourceInvalid(
        "could not reserve input capture staging storage".into(),
    ))
}

/// Lists all valid local input snapshots in ascending identifier order.
pub fn list(boundary: &RepositoryBoundary) -> Result<Vec<InputSnapshot>, InputError> {
    let root = boundary.data.join(INPUT_DIRECTORY);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_type()?.is_symlink() || !entry.file_type()?.is_dir() {
            continue;
        }
        snapshots.push(read_snapshot(&entry.path())?);
    }
    snapshots.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(snapshots)
}

/// Removes only the explicitly identified local snapshot.
pub fn remove(boundary: &RepositoryBoundary, id: &str) -> Result<(), InputError> {
    if !valid_id(id) {
        return Err(InputError::InvalidId(id.into()));
    }
    let path = boundary.data.join(INPUT_DIRECTORY).join(id);
    if !path.is_dir() {
        return Err(InputError::NotFound(id.into()));
    }
    read_snapshot(&path)?;
    fs::remove_dir_all(path)?;
    Ok(())
}

fn collect_entries(source: &Path) -> Result<Vec<SourceEntry>, InputError> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(InputError::SourceInvalid(format!(
            "symbolic links are not accepted: {}",
            source.display()
        )));
    }
    if metadata.is_file() {
        let name = source.file_name().ok_or_else(|| {
            InputError::SourceInvalid(format!("source has no file name: {}", source.display()))
        })?;
        return Ok(vec![SourceEntry {
            relative: PathBuf::from(portable_name(name)?),
            source: source.into(),
            directory: false,
        }]);
    }
    if !metadata.is_dir() {
        return Err(InputError::SourceInvalid(format!(
            "source is not a regular file or directory: {}",
            source.display()
        )));
    }
    let mut entries = Vec::new();
    collect_directory(source, Path::new(""), &mut entries)?;
    Ok(entries)
}

fn collect_directory(
    directory: &Path,
    relative: &Path,
    entries: &mut Vec<SourceEntry>,
) -> Result<(), InputError> {
    let mut children = fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            Ok::<_, std::io::Error>((portable_name(&entry.file_name()), entry))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut children = children
        .drain(..)
        .map(|(name, entry)| name.map(|name| (name, entry)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error: InputError| error)?;
    children.sort_by(|left, right| left.0.cmp(&right.0));
    for (name, child) in children {
        let child_relative = relative.join(&name);
        let kind = child.file_type()?;
        if kind.is_symlink() {
            return Err(InputError::SourceInvalid(format!(
                "symbolic links are not accepted: {}",
                child.path().display()
            )));
        }
        if kind.is_dir() {
            entries.push(SourceEntry {
                relative: child_relative.clone(),
                source: child.path(),
                directory: true,
            });
            collect_directory(&child.path(), &child_relative, entries)?;
        } else if kind.is_file() {
            entries.push(SourceEntry {
                relative: child_relative,
                source: child.path(),
                directory: false,
            });
        } else {
            return Err(InputError::SourceInvalid(format!(
                "unsupported source entry: {}",
                child.path().display()
            )));
        }
    }
    Ok(())
}

fn write_snapshot(entries: &[SourceEntry], content: &Path) -> Result<(String, u64), InputError> {
    let mut digest = Sha256::new();
    digest.update(b"kero-input\0");
    digest.update(FORMAT_VERSION.to_be_bytes());
    let mut files = 0;
    for entry in entries {
        let path = portable_path(&entry.relative)?;
        if entry.directory {
            digest.update(b"directory\0");
            digest.update(path.as_bytes());
            digest.update(b"\0");
            fs::create_dir(content.join(&entry.relative))?;
        } else {
            let bytes = fs::read(&entry.source)?;
            digest.update(b"file\0");
            digest.update(path.as_bytes());
            digest.update(b"\0");
            digest.update((bytes.len() as u64).to_be_bytes());
            digest.update(&bytes);
            if let Some(parent) = content.join(&entry.relative).parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(content.join(&entry.relative), bytes)?;
            files += 1;
        }
    }
    Ok((hex::encode(digest.finalize()), files))
}

fn read_snapshot(path: &Path) -> Result<InputSnapshot, InputError> {
    let bytes = fs::read(path.join("manifest.json"))?;
    let snapshot = serde_json::from_slice::<InputSnapshot>(&bytes)
        .map_err(|error| InputError::Manifest(format!("{}: {error}", path.display())))?;
    if !valid_id(&snapshot.id) || snapshot.format_version != FORMAT_VERSION {
        return Err(InputError::Manifest(format!(
            "unsupported snapshot manifest: {}",
            path.display()
        )));
    }
    let directory_id = path.file_name().and_then(OsStr::to_str).ok_or_else(|| {
        InputError::Manifest(format!("snapshot path is not UTF-8: {}", path.display()))
    })?;
    if directory_id != snapshot.id {
        return Err(InputError::Manifest(format!(
            "snapshot directory does not match manifest identifier: {}",
            path.display()
        )));
    }
    let (actual_id, actual_files) = digest_snapshot_content(&path.join("content"))?;
    if actual_id != snapshot.id || actual_files != snapshot.files {
        return Err(InputError::Manifest(format!(
            "snapshot content does not match manifest: {}",
            path.display()
        )));
    }
    Ok(snapshot)
}

fn digest_snapshot_content(content: &Path) -> Result<(String, u64), InputError> {
    let entries = collect_entries(content)?;
    let mut digest = Sha256::new();
    digest.update(b"kero-input\0");
    digest.update(FORMAT_VERSION.to_be_bytes());
    let mut files = 0;
    for entry in entries {
        let path = portable_path(&entry.relative)?;
        if entry.directory {
            digest.update(b"directory\0");
            digest.update(path.as_bytes());
            digest.update(b"\0");
        } else {
            let bytes = fs::read(entry.source)?;
            digest.update(b"file\0");
            digest.update(path.as_bytes());
            digest.update(b"\0");
            digest.update((bytes.len() as u64).to_be_bytes());
            digest.update(bytes);
            files += 1;
        }
    }
    Ok((hex::encode(digest.finalize()), files))
}

fn portable_path(path: &Path) -> Result<String, InputError> {
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(InputError::SourceInvalid(format!(
            "input path escapes its source: {}",
            path.display()
        )));
    }
    path.components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(portable_name(name)),
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|segments| segments.join("/"))
}

fn portable_name(name: &OsStr) -> Result<String, InputError> {
    let name = name.to_str().ok_or_else(|| {
        InputError::SourceInvalid("input paths must use valid UTF-8 names".into())
    })?;
    if name.is_empty() || name.contains(['/', '\\']) {
        return Err(InputError::SourceInvalid(format!(
            "input path has an invalid name: {name:?}"
        )));
    }
    Ok(name.into())
}

fn valid_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

/// Returns the validated content root for one explicitly identified snapshot.
pub fn content_root(boundary: &RepositoryBoundary, id: &str) -> Result<PathBuf, InputError> {
    if !valid_id(id) {
        return Err(InputError::InvalidId(id.into()));
    }
    let snapshot = boundary.data.join(INPUT_DIRECTORY).join(id);
    read_snapshot(&snapshot)?;
    Ok(snapshot.join("content"))
}
