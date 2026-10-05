//! Release-1 repository boundary and ownership operations.

use crate::config::{self, Node};
use crate::host::{DataScope, KeroHost, LifecycleHost, ScopedPath};
use crate::lifecycle;
use crate::setup::Enrollment;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// The human-owned Release-1 configuration filename.
pub const CONFIG_FILE: &str = "config";
/// Repository-local knowledge owned by this KERO environment.
pub const DATA_DIRECTORY: &str = "data";
/// Materialized external KERO environments, one immediate child per mount.
pub const MOUNT_DIRECTORY: &str = "mnt";
/// Hidden runtime state, created only by an operation that needs it.
pub const RUNTIME_DIRECTORY: &str = ".runtime";
/// The KERO-owned path that Git should ignore for an initialized environment.
pub const OWNED_DIRECTORIES: [&str; 1] = [RUNTIME_DIRECTORY];

/// The recognized shape of one explicit KERO environment candidate.
///
/// This classifier never searches outside the selected path. A caller may use
/// it before proposing a repository or global-home source for a mount.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EnvironmentFormat {
    NeedsSetup,
    Repository,
    GlobalHome,
    Conflicted,
}

/// Format facts for one explicit KERO environment candidate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EnvironmentInspection {
    pub format: EnvironmentFormat,
    pub selected_path: PathBuf,
    pub root: Option<PathBuf>,
    pub message: String,
}

/// Stable classification of one explicitly selected repository candidate.
///
/// Discovery is bounded to the selected path and its ancestors. It never
/// inventories KERO home, sibling directories, or the rest of the machine.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RepositoryState {
    NotRepository,
    Unsupported,
    Unavailable,
    Enrolled,
    Eligible,
}

/// A repository classification returned before enrollment can mutate it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RepositoryDiscovery {
    pub state: RepositoryState,
    pub selected_path: PathBuf,
    pub worktree_root: Option<PathBuf>,
    pub boundary: Option<RepositoryBoundary>,
    pub message: String,
}

/// The result of applying one enrollment policy to a discovery result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EnrollmentOutcome {
    Proposed,
    Enrolled,
    AlreadyEnrolled,
    NotEnrolled,
}

/// Runtime-only provenance for one materialized external mount.
///
/// The source path is host runtime state, never repository configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MountProvenance {
    pub name: String,
    pub source_root: PathBuf,
    pub source_content_sha256: String,
    #[serde(default)]
    pub snapshot_content_sha256: String,
    pub files: u64,
    pub source_format: String,
    pub access: String,
    #[serde(default)]
    pub refresh_mode: String,
    #[serde(default)]
    pub baseline_sha256: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub last_successful_refresh: u64,
}

/// The state of one entry directly beneath an environment's mount root.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MountState {
    Empty,
    MaterializedReadOnly,
    MaterializedReadWrite,
    Conflicted,
    Stale,
    OutOfFormat,
}

/// A materialized mount visible beneath one repository environment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MountInfo {
    pub name: String,
    pub path: PathBuf,
    pub state: MountState,
    pub message: String,
}

/// A discovered Release-1 boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RepositoryBoundary {
    pub root: PathBuf,
    pub directory: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
    pub mounts: PathBuf,
}

/// Boundary operation failures.
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("repository.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("repository.exists: {0}")]
    Exists(PathBuf),
    #[error("repository.not-found: no .kero/config boundary from {0}")]
    NotFound(PathBuf),
    #[error("repository.git: {0}")]
    Git(String),
    #[error("repository.mount-name-invalid: {0}")]
    MountName(String),
    #[error("repository.{scope}-path-escape: {path}")]
    PathEscape { scope: String, path: PathBuf },
    #[error("repository.lifecycle: {0}")]
    Lifecycle(String),
    #[error("repository.mount-source: {0}")]
    MountSource(String),
    #[error("repository.initialization-ineligible: {0}")]
    InitializationIneligible(String),
    #[error("repository.configuration: {0}")]
    Configuration(String),
}

/// Native filesystem adapter for the current command-line host.
///
/// This adapter is intentionally outside the WASM target. It maps the stable
/// scoped lifecycle interface onto the chosen repository's native paths.
struct NativeLifecycleHost {
    root: PathBuf,
    discovery_start: PathBuf,
}

impl NativeLifecycleHost {
    fn new(root: PathBuf, discovery_start: PathBuf) -> Self {
        Self {
            root,
            discovery_start,
        }
    }

    fn scoped_path(&self, path: &ScopedPath) -> PathBuf {
        let mut output = match path.scope() {
            DataScope::Config => self.root.join(".kero").join(CONFIG_FILE),
            DataScope::Local => self.root.join(".kero").join(DATA_DIRECTORY),
            DataScope::Mounts => self.root.join(".kero").join(MOUNT_DIRECTORY),
            DataScope::Mount(name) => self.root.join(".kero").join(MOUNT_DIRECTORY).join(name),
            DataScope::Runtime => self.root.join(".kero").join(RUNTIME_DIRECTORY),
        };
        for segment in path.segments() {
            output.push(segment);
        }
        output
    }
}

impl KeroHost for NativeLifecycleHost {
    type Error = std::io::Error;

    fn read(&mut self, path: &ScopedPath) -> Result<Vec<u8>, Self::Error> {
        fs::read(self.scoped_path(path))
    }
    fn write(&mut self, path: &ScopedPath, bytes: &[u8]) -> Result<(), Self::Error> {
        if matches!(path.scope(), DataScope::Mount(_)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "mounted knowledge is read-only; modify its source environment instead",
            ));
        }
        let path = self.scoped_path(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)
    }
    fn create_directory(&mut self, path: &ScopedPath) -> Result<(), Self::Error> {
        if matches!(path.scope(), DataScope::Mount(_)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "mounted knowledge is read-only; modify its source environment instead",
            ));
        }
        fs::create_dir_all(self.scoped_path(path))
    }
    fn materialize_mount(&mut self, mount: &str) -> Result<(), Self::Error> {
        fs::create_dir_all(self.root.join(".kero").join(MOUNT_DIRECTORY).join(mount))
    }
    fn execute(&mut self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Err(std::io::Error::other(
            "process capability is not granted to lifecycle",
        ))
    }
    fn network(&mut self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Err(std::io::Error::other(
            "network capability is not granted to lifecycle",
        ))
    }
    fn diagnostic(&mut self, _: &str, _: &str) {}
}

impl LifecycleHost for NativeLifecycleHost {
    fn environment_exists(&mut self) -> Result<bool, Self::Error> {
        Ok(self.root.join(".kero").join(CONFIG_FILE).is_file())
    }
    fn discover_nearest_boundary(&mut self) -> Result<bool, Self::Error> {
        let mut cursor = self.discovery_start.clone();
        if cursor.is_file() {
            cursor = cursor.parent().unwrap_or(&cursor).to_path_buf();
        }
        if !cursor.is_absolute() {
            cursor = std::env::current_dir()?.join(cursor);
        }
        loop {
            if cursor.join(".kero").join(CONFIG_FILE).is_file() {
                return Ok(true);
            }
            if !cursor.pop() {
                return Ok(false);
            }
        }
    }
    fn set_runtime_hidden(&mut self, _: bool) -> Result<(), Self::Error> {
        Ok(())
    }
    fn remove_runtime(&mut self) -> Result<(), Self::Error> {
        let runtime = self.root.join(".kero").join(RUNTIME_DIRECTORY);
        if runtime.exists() {
            fs::remove_dir_all(runtime)?;
        }
        Ok(())
    }
}

/// Classifies one explicitly selected directory without mutating it.
///
/// A missing `.kero` boundary needs setup. An incomplete boundary, malformed
/// configuration, or contradictory home/repository markers is a conflict that
/// must be repaired explicitly; discovery never deletes or recreates it.
pub fn inspect_environment(path: &Path) -> EnvironmentInspection {
    let selected_path = absolute_path(path);
    let candidate = if selected_path.is_file() {
        selected_path.parent().map(Path::to_path_buf)
    } else {
        Some(selected_path.clone())
    };
    let Some(candidate) = candidate else {
        return EnvironmentInspection {
            format: EnvironmentFormat::Conflicted,
            selected_path,
            root: None,
            message: "the selected path has no directory to inspect".into(),
        };
    };

    let repository = candidate.join(".kero");
    let home_shape = candidate.join(CONFIG_FILE).is_file()
        && candidate.join(DATA_DIRECTORY).is_dir()
        && candidate.join(MOUNT_DIRECTORY).is_dir();
    if home_shape && repository.exists() {
        return EnvironmentInspection {
            format: EnvironmentFormat::Conflicted,
            selected_path,
            root: candidate.canonicalize().ok().or(Some(candidate)),
            message: "a directory cannot be both a KERO global home and repository environment"
                .into(),
        };
    }
    if home_shape {
        return EnvironmentInspection {
            format: EnvironmentFormat::GlobalHome,
            selected_path,
            root: candidate.canonicalize().ok().or(Some(candidate)),
            message: "the selected directory contains a KERO global home".into(),
        };
    }

    if !repository.exists() {
        return EnvironmentInspection {
            format: EnvironmentFormat::NeedsSetup,
            selected_path,
            root: None,
            message: "the selected directory has no .kero boundary and needs explicit setup".into(),
        };
    }
    if !repository.is_dir()
        || !repository.join(CONFIG_FILE).is_file()
        || !repository.join(DATA_DIRECTORY).is_dir()
        || !repository.join(MOUNT_DIRECTORY).is_dir()
    {
        return EnvironmentInspection {
            format: EnvironmentFormat::Conflicted,
            selected_path,
            root: candidate.canonicalize().ok().or(Some(candidate)),
            message: "the existing .kero boundary is incomplete or has paths with conflicting types; repair it without reinitializing".into(),
        };
    }
    let root = candidate.canonicalize().ok().or(Some(candidate));
    match fs::read_to_string(repository.join(CONFIG_FILE))
        .map_err(|error| error.to_string())
        .and_then(|input| config::parse(&input).map_err(|error| error.to_string()))
    {
        Ok(_) => EnvironmentInspection {
            format: EnvironmentFormat::Repository,
            selected_path,
            root,
            message: "the selected directory contains a complete KERO repository environment with valid KERO Structured Text configuration".into(),
        },
        Err(error) => EnvironmentInspection {
            format: EnvironmentFormat::Conflicted,
            selected_path,
            root,
            message: format!("repository configuration is malformed KERO Structured Text: {error}"),
        },
    }
}

/// Classifies the repository containing one explicit path without mutation.
pub fn inspect(start: &Path) -> RepositoryDiscovery {
    let selected_path = absolute_path(start);
    let output = match Command::new("git")
        .args(["-C"])
        .arg(&selected_path)
        .args(["rev-parse", "--is-inside-work-tree", "--is-bare-repository"])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            return RepositoryDiscovery {
                state: RepositoryState::Unavailable,
                selected_path,
                worktree_root: None,
                boundary: None,
                message: format!("Git could not be invoked: {error}"),
            };
        }
    };

    if !output.status.success() {
        return RepositoryDiscovery {
            state: RepositoryState::NotRepository,
            selected_path,
            worktree_root: None,
            boundary: None,
            message: "the selected path is not in a Git worktree".into(),
        };
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines().map(str::trim).collect::<Vec<_>>();
    if lines.len() < 2 || lines[1] == "true" {
        return RepositoryDiscovery {
            state: RepositoryState::Unsupported,
            selected_path,
            worktree_root: None,
            boundary: None,
            message: "the selected Git target does not provide a usable non-bare worktree".into(),
        };
    }
    if lines[0] != "true" {
        return RepositoryDiscovery {
            state: RepositoryState::NotRepository,
            selected_path,
            worktree_root: None,
            boundary: None,
            message: "the selected path is not in a Git worktree".into(),
        };
    }
    let root_output = match Command::new("git")
        .args(["-C"])
        .arg(&selected_path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(_) => {
            return RepositoryDiscovery {
                state: RepositoryState::Unavailable,
                selected_path,
                worktree_root: None,
                boundary: None,
                message: "Git could not resolve the selected worktree root".into(),
            };
        }
        Err(error) => {
            return RepositoryDiscovery {
                state: RepositoryState::Unavailable,
                selected_path,
                worktree_root: None,
                boundary: None,
                message: format!("Git could not resolve the selected worktree root: {error}"),
            };
        }
    };
    let root = String::from_utf8_lossy(&root_output.stdout)
        .trim()
        .to_owned();
    if root.is_empty() {
        return RepositoryDiscovery {
            state: RepositoryState::Unavailable,
            selected_path,
            worktree_root: None,
            boundary: None,
            message: "Git returned an empty worktree root".into(),
        };
    }
    let worktree_root = PathBuf::from(root);
    if !worktree_root.is_dir() {
        return RepositoryDiscovery {
            state: RepositoryState::Unavailable,
            selected_path,
            worktree_root: Some(worktree_root),
            boundary: None,
            message: "Git returned a worktree root that is unavailable to this host".into(),
        };
    }
    let worktree_root = worktree_root.canonicalize().unwrap_or(worktree_root);
    if let Some(boundary) = find_boundary_within(&selected_path, &worktree_root) {
        return RepositoryDiscovery {
            state: RepositoryState::Enrolled,
            selected_path,
            worktree_root: Some(worktree_root),
            boundary: Some(boundary),
            message: "the selected path belongs to an existing KERO environment".into(),
        };
    }
    if fs::metadata(&worktree_root)
        .map(|metadata| metadata.permissions().readonly())
        .unwrap_or(true)
    {
        return RepositoryDiscovery {
            state: RepositoryState::Unavailable,
            selected_path,
            worktree_root: Some(worktree_root),
            boundary: None,
            message: "the selected worktree is not writable by this host".into(),
        };
    }
    RepositoryDiscovery {
        state: RepositoryState::Eligible,
        selected_path,
        worktree_root: Some(worktree_root),
        boundary: None,
        message: "the selected worktree is eligible for KERO enrollment".into(),
    }
}

/// Applies a configured enrollment policy to one explicit repository target.
///
/// `Ask` and `Manual` are deliberately side-effect free. `Automatic` only
/// initializes the inspected worktree when that worktree is eligible.
pub fn enroll(
    start: &Path,
    policy: Enrollment,
) -> Result<(RepositoryDiscovery, EnrollmentOutcome), RepositoryError> {
    let discovery = inspect(start);
    let outcome = match discovery.state {
        RepositoryState::Enrolled => EnrollmentOutcome::AlreadyEnrolled,
        RepositoryState::Eligible if policy == Enrollment::Automatic => {
            let root = discovery.worktree_root.as_deref().ok_or_else(|| {
                RepositoryError::Lifecycle("eligible discovery omitted its worktree root".into())
            })?;
            initialize(root)?;
            EnrollmentOutcome::Enrolled
        }
        RepositoryState::Eligible if policy == Enrollment::Ask => EnrollmentOutcome::Proposed,
        _ => EnrollmentOutcome::NotEnrolled,
    };
    Ok((discovery, outcome))
}

fn absolute_path(path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    path.canonicalize().unwrap_or(path)
}

fn find_boundary_within(start: &Path, worktree_root: &Path) -> Option<RepositoryBoundary> {
    let mut cursor = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if cursor.join(".kero").join(CONFIG_FILE).is_file() {
            return Some(boundary_at(cursor));
        }
        if cursor == worktree_root || !cursor.pop() {
            return None;
        }
    }
}

/// Initializes an explicitly selected eligible Git worktree without runtime directories.
///
/// Repository environments always belong to non-bare worktrees. Callers that
/// need a global KERO home use the separate setup boundary instead.
pub fn initialize(root: &Path) -> Result<RepositoryBoundary, RepositoryError> {
    let discovery = inspect(root);
    let root = match discovery.state {
        RepositoryState::Eligible => discovery.worktree_root.ok_or_else(|| {
            RepositoryError::InitializationIneligible(
                "eligible repository discovery omitted its worktree root".into(),
            )
        })?,
        RepositoryState::Enrolled => {
            return discovery.boundary.ok_or_else(|| {
                RepositoryError::InitializationIneligible(
                    "enrolled repository discovery omitted its KERO boundary".into(),
                )
            });
        }
        state => {
            return Err(RepositoryError::InitializationIneligible(format!(
                "selected path is {state:?}; select an eligible non-bare Git worktree"
            )));
        }
    };
    let mut host = NativeLifecycleHost::new(root.clone(), root.clone());
    lifecycle::initialize(&mut host)
        .map_err(|error| RepositoryError::Lifecycle(error.to_string()))?;
    let boundary = boundary_at(root);
    install_git_excludes(&boundary)?;
    Ok(boundary)
}

/// Finds the nearest valid Release-1 config boundary.
pub fn discover(start: &Path) -> Result<RepositoryBoundary, RepositoryError> {
    let original = start.to_path_buf();
    let root = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()?.join(start)
    };
    let mut host = NativeLifecycleHost::new(root, start.to_path_buf());
    lifecycle::discover(&mut host).map_err(|error| match error {
        lifecycle::LifecycleError::NotFound => RepositoryError::NotFound(original.clone()),
        error => RepositoryError::Lifecycle(error.to_string()),
    })?;
    let mut cursor = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    if !cursor.is_absolute() {
        cursor = std::env::current_dir()?.join(cursor);
    }
    loop {
        let directory = cursor.join(".kero");
        let config = directory.join(CONFIG_FILE);
        if config.is_file() {
            return Ok(RepositoryBoundary {
                root: cursor.canonicalize()?,
                directory,
                config,
                data: cursor.join(".kero").join(DATA_DIRECTORY),
                mounts: cursor.join(".kero").join(MOUNT_DIRECTORY),
            });
        }
        if !cursor.pop() {
            return Err(RepositoryError::NotFound(original));
        }
    }
}

fn boundary_at(root: PathBuf) -> RepositoryBoundary {
    let directory = root.join(".kero");
    RepositoryBoundary {
        config: directory.join(CONFIG_FILE),
        data: directory.join(DATA_DIRECTORY),
        mounts: directory.join(MOUNT_DIRECTORY),
        root,
        directory,
    }
}

/// Resolves a repository-local knowledge path beneath `.kero/data`.
pub fn resolve_local(
    boundary: &RepositoryBoundary,
    relative: &Path,
) -> Result<PathBuf, RepositoryError> {
    resolve_under(&boundary.data, relative, "local")
}

/// Resolves a mounted knowledge path beneath `.kero/mnt/<mount>`.
pub fn resolve_mount(
    boundary: &RepositoryBoundary,
    mount: &str,
    relative: &Path,
) -> Result<PathBuf, RepositoryError> {
    if !valid_mount_name(mount) {
        return Err(RepositoryError::MountName(mount.into()));
    }
    resolve_under(&boundary.mounts.join(mount), relative, "mount")
}

/// Creates an empty external-mount materialization directory.
///
/// Transfer, synchronization, and authentication are host responsibilities;
/// this operation records no source path or machine-specific detail in config.
pub fn create_mount(
    boundary: &RepositoryBoundary,
    mount: &str,
) -> Result<PathBuf, RepositoryError> {
    if !valid_mount_name(mount) {
        return Err(RepositoryError::MountName(mount.into()));
    }
    let path = boundary.mounts.join(mount);
    fs::create_dir(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            RepositoryError::Exists(path.clone())
        } else {
            error.into()
        }
    })?;
    Ok(path)
}

/// Lists and classifies the immediate entries beneath one environment's mount
/// root. Invalid or incomplete entries remain visible as `outOfFormat` rather
/// than disappearing from the terminal view.
pub fn list_mounts(boundary: &RepositoryBoundary) -> Result<Vec<MountInfo>, RepositoryError> {
    let mut mounts = Vec::new();
    for entry in fs::read_dir(&boundary.mounts)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".gitkeep" {
            continue;
        }
        let path = entry.path();
        let (state, message) = if kind.is_symlink() || !kind.is_dir() {
            (
                MountState::OutOfFormat,
                "a mount entry must be a real directory".into(),
            )
        } else if !valid_mount_name(&name) {
            (
                MountState::OutOfFormat,
                "mount names use lower-case letters, digits, and hyphens".into(),
            )
        } else {
            match read_mount_provenance(boundary, &name).ok().flatten() {
                Some(record) if record.status == "conflicted" => (
                    MountState::Conflicted,
                    "source and mounted data both changed; choose a conflict resolution".into(),
                ),
                Some(record) if record.status == "stale" => (
                    MountState::Stale,
                    "the last refresh failed; the prior complete snapshot remains visible".into(),
                ),
                Some(record) if record.access == "read-only" => (
                    MountState::MaterializedReadOnly,
                    format!(
                        "read-only materialization of {}",
                        record.source_root.display()
                    ),
                ),
                Some(record) if record.access == "read-write" => (
                    MountState::MaterializedReadWrite,
                    format!(
                        "grant-authorized writable synchronization with {}",
                        record.source_root.display()
                    ),
                ),
                Some(_) => (
                    MountState::OutOfFormat,
                    "mount provenance has an unsupported access mode".into(),
                ),
                None if fs::read_dir(&path)?.next().is_none() => (
                    MountState::Empty,
                    "empty mount placeholder; no external environment has been materialized".into(),
                ),
                None => (
                    MountState::OutOfFormat,
                    "mount contents have no valid KERO provenance record".into(),
                ),
            }
        };
        mounts.push(MountInfo {
            name,
            path,
            state,
            message,
        });
    }
    mounts.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(mounts)
}

/// Materializes one explicit external KERO environment into an isolated mount.
///
/// Only the source environment's local `data/` is copied. The host-local source
/// location and a deterministic content digest are recorded beneath `.runtime`
/// so repository configuration remains portable and contains no transport path.
pub fn materialize_mount(
    boundary: &RepositoryBoundary,
    mount: &str,
    source: &Path,
) -> Result<MountProvenance, RepositoryError> {
    if !valid_mount_name(mount) {
        return Err(RepositoryError::MountName(mount.into()));
    }
    let inspection = inspect_environment(source);
    if inspection.format != EnvironmentFormat::Repository {
        return Err(RepositoryError::MountSource(format!(
            "source must be an in-format KERO repository environment: {}",
            inspection.message
        )));
    }
    let source_root = inspection.root.ok_or_else(|| {
        RepositoryError::MountSource("repository inspection omitted its source root".into())
    })?;
    if source_root == boundary.root {
        return Err(RepositoryError::MountSource(
            "a repository cannot mount its own local data".into(),
        ));
    }
    materialize_snapshot(
        boundary,
        mount,
        source_root.join(".kero").join(DATA_DIRECTORY),
        source_root,
        "repository",
    )
}

/// Materializes the selected marked global home's data as a read-only mount.
///
/// A global home is never ambient repository input. The caller must select it
/// explicitly, and only its `data/` tree is copied into the target mount.
pub fn materialize_global_home(
    boundary: &RepositoryBoundary,
    mount: &str,
    source: &Path,
) -> Result<MountProvenance, RepositoryError> {
    if !valid_mount_name(mount) {
        return Err(RepositoryError::MountName(mount.into()));
    }
    let inspection = inspect_environment(source);
    if inspection.format != EnvironmentFormat::GlobalHome {
        return Err(RepositoryError::MountSource(format!(
            "source must be a marked KERO global home: {}",
            inspection.message
        )));
    }
    let source_root = inspection.root.ok_or_else(|| {
        RepositoryError::MountSource("global-home inspection omitted its source root".into())
    })?;
    materialize_snapshot(
        boundary,
        mount,
        source_root.join(DATA_DIRECTORY),
        source_root,
        "global-home",
    )
}

/// Builds a complete read-only snapshot in disposable runtime state before
/// atomically publishing it beneath the visible mount root.
fn materialize_snapshot(
    boundary: &RepositoryBoundary,
    mount: &str,
    source_data: PathBuf,
    source_root: PathBuf,
    source_format: &str,
) -> Result<MountProvenance, RepositoryError> {
    if !source_data.is_dir() {
        return Err(RepositoryError::MountSource(format!(
            "source local data directory is missing: {}",
            source_data.display()
        )));
    }
    let destination = boundary.mounts.join(mount);
    if destination.exists() {
        return Err(RepositoryError::Exists(destination));
    }

    let staging_directory = boundary
        .directory
        .join(RUNTIME_DIRECTORY)
        .join("mount-staging");
    fs::create_dir_all(&staging_directory)?;
    let staging = staging_directory.join(mount);
    fs::create_dir(&staging).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            RepositoryError::Exists(staging.clone())
        } else {
            error.into()
        }
    })?;

    let mut published = false;
    let result = (|| {
        let destination_case_sensitive = destination_is_case_sensitive(&staging)?;
        let (digest, files) = copy_materialized_tree(
            &source_data,
            &staging,
            Path::new(""),
            destination_case_sensitive,
        )?;
        let staging_permissions = fs::metadata(&staging)?.permissions();
        set_tree_readonly(&staging, true)?;
        // The root must stay writable until rename on some Unix filesystems.
        fs::set_permissions(&staging, staging_permissions.clone())?;
        let provenance = MountProvenance {
            name: mount.into(),
            source_root,
            source_content_sha256: digest.clone(),
            snapshot_content_sha256: digest.clone(),
            files,
            source_format: source_format.into(),
            access: "read-only".into(),
            refresh_mode: "manual".into(),
            baseline_sha256: digest.clone(),
            status: "ready".into(),
            last_successful_refresh: unix_seconds()?,
        };
        let metadata_directory = boundary.directory.join(RUNTIME_DIRECTORY).join("mounts");
        fs::create_dir_all(&metadata_directory)?;
        let metadata_path = metadata_directory.join(format!("{mount}.kst"));
        fs::write(&metadata_path, encode_mount_provenance(&provenance))?;
        if let Err(error) = fs::rename(&staging, &destination) {
            let _ = fs::remove_file(&metadata_path);
            return Err(RepositoryError::Io(error));
        }
        published = true;
        let mut readonly_permissions = staging_permissions;
        readonly_permissions.set_readonly(true);
        if let Err(error) = fs::set_permissions(&destination, readonly_permissions) {
            let _ = fs::remove_file(&metadata_path);
            return Err(RepositoryError::Io(error));
        }
        Ok(provenance)
    })();
    if result.is_err() {
        let cleanup = if published { &destination } else { &staging };
        let _ = set_tree_readonly(cleanup, false);
        let _ = fs::remove_dir_all(cleanup);
    }
    result
}

/// Removes one mount and its disposable runtime provenance without touching
/// repository-local knowledge or any sibling mount.
pub fn remove_mount(boundary: &RepositoryBoundary, mount: &str) -> Result<(), RepositoryError> {
    if !valid_mount_name(mount) {
        return Err(RepositoryError::MountName(mount.into()));
    }
    let mount_path = boundary.mounts.join(mount);
    if !mount_path.is_dir() {
        return Err(RepositoryError::NotFound(mount_path));
    }
    set_tree_readonly(&mount_path, false)?;
    fs::remove_dir_all(&mount_path)?;
    for extension in ["kst", "json"] {
        let metadata = boundary
            .directory
            .join(RUNTIME_DIRECTORY)
            .join("mounts")
            .join(format!("{mount}.{extension}"));
        if metadata.is_file() {
            fs::remove_file(metadata)?;
        }
    }
    Ok(())
}

/// Rebuilds a mount from its recorded explicit source and replaces the prior
/// snapshot only after the new snapshot validates successfully.
pub fn refresh_mount(
    boundary: &RepositoryBoundary,
    mount: &str,
) -> Result<MountProvenance, RepositoryError> {
    let record = read_mount_provenance(boundary, mount)?.ok_or_else(|| {
        RepositoryError::NotFound(
            boundary
                .directory
                .join(RUNTIME_DIRECTORY)
                .join("mounts")
                .join(format!("{mount}.kst")),
        )
    })?;
    let source_data = match record.source_format.as_str() {
        "repository" => record.source_root.join(".kero").join(DATA_DIRECTORY),
        "global-home" => record.source_root.join(DATA_DIRECTORY),
        _ => {
            return Err(RepositoryError::MountSource(
                "mount provenance has an unsupported source format".into(),
            ));
        }
    };
    let destination = boundary.mounts.join(mount);
    if !destination.is_dir() {
        return Err(RepositoryError::NotFound(destination));
    }
    let runtime = boundary.directory.join(RUNTIME_DIRECTORY);
    fs::create_dir_all(runtime.join("mount-staging"))?;
    let backup = runtime
        .join("mount-staging")
        .join(format!("{mount}.previous"));
    if backup.exists() {
        return Err(RepositoryError::Exists(backup));
    }
    let metadata = runtime.join("mounts").join(format!("{mount}.kst"));
    let prior_metadata = fs::read(&metadata).ok();
    set_tree_readonly(&destination, false)?;
    fs::rename(&destination, &backup)?;
    match materialize_snapshot(
        boundary,
        mount,
        source_data,
        record.source_root.clone(),
        &record.source_format,
    ) {
        Ok(mut next) => {
            next.refresh_mode = record.refresh_mode;
            next.access = record.access;
            next.baseline_sha256 = next.source_content_sha256.clone();
            next.status = "ready".into();
            next.last_successful_refresh = unix_seconds()?;
            if next.access == "read-write" {
                set_tree_readonly(&boundary.mounts.join(mount), false)?;
            }
            fs::write(&metadata, encode_mount_provenance(&next))?;
            let _ = fs::remove_dir_all(&backup);
            let _ = fs::remove_file(runtime.join("mounts").join(format!("{mount}.json")));
            Ok(next)
        }
        Err(error) => {
            let _ = fs::rename(&backup, &destination);
            if prior_metadata.is_some() {
                let mut stale = record;
                stale.status = "stale".into();
                let _ = fs::write(&metadata, encode_mount_provenance(&stale));
            }
            Err(error)
        }
    }
}

/// Returns the explicit data directory recorded for a materialized mount.
/// The path is runtime-only provenance and is suitable only for host watchers.
pub fn mount_source_data(
    boundary: &RepositoryBoundary,
    mount: &str,
) -> Result<PathBuf, RepositoryError> {
    let record = read_mount_provenance(boundary, mount)?.ok_or_else(|| {
        RepositoryError::NotFound(
            boundary
                .directory
                .join(RUNTIME_DIRECTORY)
                .join("mounts")
                .join(format!("{mount}.kst")),
        )
    })?;
    match record.source_format.as_str() {
        "repository" => Ok(record.source_root.join(".kero").join(DATA_DIRECTORY)),
        "global-home" => Ok(record.source_root.join(DATA_DIRECTORY)),
        _ => Err(RepositoryError::MountSource(
            "mount provenance has an unsupported source format".into(),
        )),
    }
}

/// Reads the disposable provenance for a named materialized mount.
pub fn mount_provenance(
    boundary: &RepositoryBoundary,
    mount: &str,
) -> Result<MountProvenance, RepositoryError> {
    read_mount_provenance(boundary, mount)?
        .ok_or_else(|| RepositoryError::NotFound(boundary.mounts.join(mount)))
}

/// Changes the durable refresh policy for a materialized mount. The source
/// remains explicit in runtime provenance; no policy can create a new mount.
pub fn set_mount_refresh_mode(
    boundary: &RepositoryBoundary,
    mount: &str,
    refresh_mode: &str,
) -> Result<MountProvenance, RepositoryError> {
    if !matches!(refresh_mode, "manual" | "event") {
        return Err(RepositoryError::Configuration(
            "mount refresh must be manual or event".into(),
        ));
    }
    let mut record = mount_provenance(boundary, mount)?;
    record.refresh_mode = refresh_mode.into();
    fs::write(
        boundary
            .directory
            .join(RUNTIME_DIRECTORY)
            .join("mounts")
            .join(format!("{mount}.kst")),
        encode_mount_provenance(&record),
    )?;
    Ok(record)
}

/// Synchronizes one grant-authorized local mount without resolving divergent
/// changes automatically. The caller supplies its public identity key.
pub fn sync_mount(
    boundary: &RepositoryBoundary,
    mount: &str,
    target_key: &str,
) -> Result<MountProvenance, RepositoryError> {
    let mut record = read_mount_provenance(boundary, mount)?
        .ok_or_else(|| RepositoryError::NotFound(boundary.mounts.join(mount)))?;
    let source_data = mount_source_data(boundary, mount)?;
    let source_config = if record.source_format == "repository" {
        record.source_root.join(".kero").join(CONFIG_FILE)
    } else {
        record.source_root.join(CONFIG_FILE)
    };
    let direction = verify_sync_grant(&source_config, mount, target_key, &record.baseline_sha256)?;
    let source_digest = digest_tree(&source_data)?;
    let local_path = boundary.mounts.join(mount);
    let local_digest = digest_tree(&local_path)?;
    let source_changed = source_digest != record.baseline_sha256;
    let local_changed = local_digest != record.baseline_sha256;
    if source_changed && local_changed {
        record.status = "conflicted".into();
        fs::write(
            boundary
                .directory
                .join(RUNTIME_DIRECTORY)
                .join("mounts")
                .join(format!("{mount}.kst")),
            encode_mount_provenance(&record),
        )?;
        return Err(RepositoryError::MountSource("mount.sync-conflict: source and mounted data changed since the last synchronized snapshot".into()));
    }
    if source_changed {
        if direction == "push" {
            return Err(RepositoryError::MountSource(
                "mount.sync-denied: grant does not allow pull".into(),
            ));
        }
        return refresh_mount(boundary, mount);
    }
    if local_changed {
        if direction == "pull" {
            return Err(RepositoryError::MountSource(
                "mount.sync-denied: grant does not allow push".into(),
            ));
        }
        replace_source_tree(&source_data, &local_path, &record.source_root)?;
        record.source_content_sha256 = local_digest.clone();
        record.snapshot_content_sha256 = local_digest.clone();
        record.baseline_sha256 = local_digest;
        record.access = "read-write".into();
        record.status = "ready".into();
        record.last_successful_refresh = unix_seconds()?;
        let metadata = boundary
            .directory
            .join(RUNTIME_DIRECTORY)
            .join("mounts")
            .join(format!("{mount}.kst"));
        fs::write(metadata, encode_mount_provenance(&record))?;
        return Ok(record);
    }
    if direction != "pull" {
        set_tree_readonly(&local_path, false)?;
        record.access = "read-write".into();
        fs::write(
            boundary
                .directory
                .join(RUNTIME_DIRECTORY)
                .join("mounts")
                .join(format!("{mount}.kst")),
            encode_mount_provenance(&record),
        )?;
    }
    Ok(record)
}

/// Exports complete source and mounted trees for a blocked conflict. The
/// destination must be outside the mount and is created only when absent.
pub fn export_mount_conflict(
    boundary: &RepositoryBoundary,
    mount: &str,
    output: &Path,
) -> Result<PathBuf, RepositoryError> {
    if output.exists() {
        return Err(RepositoryError::Exists(output.into()));
    }
    let source = mount_source_data(boundary, mount)?;
    let local = boundary.mounts.join(mount);
    if !local.is_dir() {
        return Err(RepositoryError::NotFound(local));
    }
    fs::create_dir_all(output)?;
    let result = (|| {
        let source_out = output.join("source");
        let local_out = output.join("mounted");
        fs::create_dir(&source_out)?;
        fs::create_dir(&local_out)?;
        copy_materialized_tree(
            &source,
            &source_out,
            Path::new(""),
            destination_is_case_sensitive(&source_out)?,
        )?;
        copy_materialized_tree(
            &local,
            &local_out,
            Path::new(""),
            destination_is_case_sensitive(&local_out)?,
        )?;
        fs::write(
            output.join("README.kst"),
            format!(
                "# KERO conflict export; merge manually, then choose source or local.\nmount {mount}\nsourceRoot {:?}\n",
                source.display().to_string()
            ),
        )?;
        Ok(output.to_path_buf())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(output);
    }
    result
}

/// Resolves a blocked conflict only after an explicit user choice.
pub fn resolve_mount_conflict(
    boundary: &RepositoryBoundary,
    mount: &str,
    target_key: &str,
    use_local: bool,
) -> Result<MountProvenance, RepositoryError> {
    if !use_local {
        return refresh_mount(boundary, mount);
    }
    let mut record = read_mount_provenance(boundary, mount)?
        .ok_or_else(|| RepositoryError::NotFound(boundary.mounts.join(mount)))?;
    let source_config = if record.source_format == "repository" {
        record.source_root.join(".kero").join(CONFIG_FILE)
    } else {
        record.source_root.join(CONFIG_FILE)
    };
    let direction = verify_sync_grant(&source_config, mount, target_key, &record.baseline_sha256)?;
    if direction == "pull" {
        return Err(RepositoryError::MountSource(
            "mount.sync-denied: grant does not allow push".into(),
        ));
    }
    let local = boundary.mounts.join(mount);
    let digest = digest_tree(&local)?;
    replace_source_tree(
        &mount_source_data(boundary, mount)?,
        &local,
        &record.source_root,
    )?;
    record.source_content_sha256 = digest.clone();
    record.snapshot_content_sha256 = digest.clone();
    record.baseline_sha256 = digest;
    record.status = "ready".into();
    record.access = "read-write".into();
    record.last_successful_refresh = unix_seconds()?;
    fs::write(
        boundary
            .directory
            .join(RUNTIME_DIRECTORY)
            .join("mounts")
            .join(format!("{mount}.kst")),
        encode_mount_provenance(&record),
    )?;
    Ok(record)
}

fn verify_sync_grant(
    config_path: &Path,
    mount: &str,
    target_key: &str,
    baseline: &str,
) -> Result<String, RepositoryError> {
    let document = config::parse(&fs::read_to_string(config_path)?)
        .map_err(|error| RepositoryError::MountSource(error.to_string()))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RepositoryError::MountSource("system clock is before Unix epoch".into()))?
        .as_secs();
    if document.nodes.iter().any(|node| {
        node.name == "syncGrantRevocation"
            && node.value.as_deref() == Some(mount)
            && node.children.iter().any(|child| {
                child.name == "targetKey" && child.value.as_deref() == Some(target_key)
            })
    }) {
        return Err(RepositoryError::MountSource(
            "mount.sync-denied: the source owner revoked this grant".into(),
        ));
    }
    for grant in document
        .nodes
        .iter()
        .filter(|node| node.name == "syncGrant" && node.value.as_deref() == Some(mount))
    {
        let child = |name: &str| {
            grant
                .children
                .iter()
                .find(|node| node.name == name)
                .and_then(|node| node.value.clone())
        };
        let (
            Some(key),
            Some(direction),
            Some(grant_baseline),
            Some(expires),
            Some(source_key),
            Some(signature),
        ) = (
            child("targetKey"),
            child("direction"),
            child("baseline"),
            child("expires"),
            child("sourceKey"),
            child("signature"),
        )
        else {
            continue;
        };
        if key != target_key
            || grant_baseline != baseline
            || expires
                .parse::<u64>()
                .ok()
                .filter(|value| *value >= now)
                .is_none()
        {
            continue;
        }
        if !matches!(direction.as_str(), "pull" | "push" | "bidirectional") {
            continue;
        }
        let payload = format!(
            "mount={mount}\ntarget={key}\ndirection={direction}\nbaseline={grant_baseline}\nexpires={expires}\n"
        );
        let public: [u8; 32] = hex::decode(source_key)
            .map_err(|_| RepositoryError::MountSource("grant source key is invalid".into()))?
            .try_into()
            .map_err(|_| RepositoryError::MountSource("grant source key is invalid".into()))?;
        let signature: [u8; 64] = hex::decode(signature)
            .map_err(|_| RepositoryError::MountSource("grant signature is invalid".into()))?
            .try_into()
            .map_err(|_| RepositoryError::MountSource("grant signature is invalid".into()))?;
        VerifyingKey::from_bytes(&public)
            .map_err(|_| RepositoryError::MountSource("grant source key is invalid".into()))?
            .verify(payload.as_bytes(), &Signature::from_bytes(&signature))
            .map_err(|_| {
                RepositoryError::MountSource("grant signature verification failed".into())
            })?;
        return Ok(direction);
    }
    Err(RepositoryError::MountSource(
        "mount.sync-denied: no valid source grant for this identity and mount".into(),
    ))
}

fn digest_tree(root: &Path) -> Result<String, RepositoryError> {
    Ok(hash_materialized_tree(root, Path::new(""))?.0)
}

/// Validates and hashes a tree using the same deterministic representation as
/// publication without copying it to another filesystem location.
fn hash_materialized_tree(
    root: &Path,
    relative_root: &Path,
) -> Result<(String, u64), RepositoryError> {
    let mut entries = fs::read_dir(root)
        .map_err(|error| mount_copy_error(relative_root, "read directory", error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| mount_copy_error(relative_root, "read directory entry", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut digest = Sha256::new();
    let mut files = 0;
    for entry in entries {
        let name = entry.file_name();
        let relative = relative_root.join(&name);
        validate_destination_name(&name, &relative)?;
        let kind = entry
            .file_type()
            .map_err(|error| mount_copy_error(&relative, "read entry type", error))?;
        if kind.is_symlink() {
            return Err(RepositoryError::MountSource(format!(
                "source data contains a symbolic link at {}",
                relative.display()
            )));
        }
        if kind.is_dir() {
            let (child, count) = hash_materialized_tree(&entry.path(), &relative)?;
            digest.update(b"directory\0");
            digest.update(relative.to_string_lossy().as_bytes());
            digest.update(b"\0");
            digest.update(child.as_bytes());
            files += count;
        } else if kind.is_file() {
            let bytes = fs::read(entry.path())
                .map_err(|error| mount_copy_error(&relative, "read file", error))?;
            digest.update(b"file\0");
            digest.update(relative.to_string_lossy().as_bytes());
            digest.update(b"\0");
            digest.update(bytes);
            files += 1;
        } else {
            return Err(RepositoryError::MountSource(format!(
                "source data contains an unsupported entry at {}",
                relative.display()
            )));
        }
    }
    Ok((hex::encode(digest.finalize()), files))
}

fn replace_source_tree(
    source: &Path,
    local: &Path,
    source_root: &Path,
) -> Result<(), RepositoryError> {
    let runtime = source_root.join(RUNTIME_DIRECTORY).join("sync-staging");
    fs::create_dir_all(&runtime)?;
    let staged = runtime.join("next");
    let backup = runtime.join("previous");
    if staged.exists() || backup.exists() {
        return Err(RepositoryError::Exists(runtime));
    }
    fs::create_dir(&staged)?;
    copy_materialized_tree(
        local,
        &staged,
        Path::new(""),
        destination_is_case_sensitive(&staged)?,
    )?;
    fs::rename(source, &backup)?;
    if let Err(error) = fs::rename(&staged, source) {
        let _ = fs::rename(&backup, source);
        return Err(error.into());
    }
    fs::remove_dir_all(backup)?;
    Ok(())
}

fn read_mount_provenance(
    boundary: &RepositoryBoundary,
    mount: &str,
) -> Result<Option<MountProvenance>, RepositoryError> {
    let directory = boundary.directory.join(RUNTIME_DIRECTORY).join("mounts");
    let kst = directory.join(format!("{mount}.kst"));
    if kst.is_file() {
        return decode_mount_provenance(&fs::read_to_string(kst)?).map(Some);
    }
    let legacy = directory.join(format!("{mount}.json"));
    if !legacy.is_file() {
        return Ok(None);
    }
    let mut record: MountProvenance =
        serde_json::from_slice(&fs::read(legacy)?).map_err(|error| {
            RepositoryError::Lifecycle(format!("legacy mount provenance is invalid: {error}"))
        })?;
    if record.refresh_mode.is_empty() {
        record.refresh_mode = "manual".into();
    }
    if record.baseline_sha256.is_empty() {
        record.baseline_sha256 = record.source_content_sha256.clone();
    }
    if record.snapshot_content_sha256.is_empty() {
        record.snapshot_content_sha256 = record.source_content_sha256.clone();
    }
    if record.status.is_empty() {
        record.status = "ready".into();
    }
    Ok(Some(record))
}

fn encode_mount_provenance(record: &MountProvenance) -> String {
    format!(
        "# Generated KERO mount provenance. Do not edit; runtime state is disposable.\nname {}\nsourceRoot {:?}\nsourceContentSha256 {}\nsnapshotContentSha256 {}\nfiles {}\nsourceFormat {}\naccess {}\nrefreshMode {}\nbaselineSha256 {}\nlastSuccessfulRefresh {}\nstatus {}\n",
        record.name,
        record.source_root.display().to_string(),
        record.source_content_sha256,
        record.snapshot_content_sha256,
        record.files,
        record.source_format,
        record.access,
        record.refresh_mode,
        record.baseline_sha256,
        record.last_successful_refresh,
        record.status
    )
}

fn decode_mount_provenance(input: &str) -> Result<MountProvenance, RepositoryError> {
    let document =
        config::parse(input).map_err(|error| RepositoryError::Lifecycle(error.to_string()))?;
    let value = |name: &str| -> Result<String, RepositoryError> {
        document
            .nodes
            .iter()
            .find(|node: &&Node| node.name == name)
            .and_then(|node| node.value.clone())
            .ok_or_else(|| {
                RepositoryError::Lifecycle(format!("mount provenance is missing {name}"))
            })
    };
    let source_content_sha256 = value("sourceContentSha256")?;
    let snapshot_content_sha256 = document
        .nodes
        .iter()
        .find(|node| node.name == "snapshotContentSha256")
        .and_then(|node| node.value.clone())
        .unwrap_or_else(|| source_content_sha256.clone());
    Ok(MountProvenance {
        name: value("name")?,
        source_root: PathBuf::from(value("sourceRoot")?),
        source_content_sha256,
        snapshot_content_sha256,
        files: value("files")?
            .parse()
            .map_err(|_| RepositoryError::Lifecycle("mount provenance has invalid files".into()))?,
        source_format: value("sourceFormat")?,
        access: value("access")?,
        refresh_mode: value("refreshMode")?,
        baseline_sha256: value("baselineSha256")?,
        last_successful_refresh: document
            .nodes
            .iter()
            .find(|node| node.name == "lastSuccessfulRefresh")
            .and_then(|node| node.value.clone())
            .unwrap_or_default()
            .parse()
            .unwrap_or(0),
        status: value("status")?,
    })
}

fn unix_seconds() -> Result<u64, RepositoryError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RepositoryError::Lifecycle("system clock is before Unix epoch".into()))
        .map(|time| time.as_secs())
}

fn copy_materialized_tree(
    source: &Path,
    destination: &Path,
    relative_root: &Path,
    destination_case_sensitive: bool,
) -> Result<(String, u64), RepositoryError> {
    let mut entries = fs::read_dir(source)
        .map_err(|error| mount_copy_error(relative_root, "read directory", error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| mount_copy_error(relative_root, "read directory entry", error))?;
    entries.sort_by_key(|entry| entry.file_name());
    if !destination_case_sensitive {
        let mut names = std::collections::BTreeSet::new();
        for entry in &entries {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if !names.insert(name) {
                return Err(RepositoryError::MountSource(format!(
                    "source data contains names that collide on the destination filesystem at {}",
                    relative_root.display()
                )));
            }
        }
    }
    let mut digest = Sha256::new();
    let mut files = 0;
    for entry in entries {
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let relative = PathBuf::from(&name);
        let source_relative = relative_root.join(&relative);
        validate_destination_name(&name, &source_relative)?;
        if file_type.is_symlink() {
            return Err(RepositoryError::MountSource(format!(
                "source data contains a symbolic link at {}",
                source_relative.display()
            )));
        }
        if file_type.is_dir() {
            let target = destination.join(&name);
            fs::create_dir(&target)
                .map_err(|error| mount_copy_error(&source_relative, "create directory", error))?;
            let (child_digest, child_files) = copy_materialized_tree(
                &entry.path(),
                &target,
                &source_relative,
                destination_case_sensitive,
            )?;
            copy_modified_time(&entry.path(), &target, &source_relative)?;
            digest.update(b"directory\0");
            digest.update(relative.to_string_lossy().as_bytes());
            digest.update(b"\0");
            digest.update(child_digest.as_bytes());
            files += child_files;
        } else if file_type.is_file() {
            let bytes = fs::read(entry.path())
                .map_err(|error| mount_copy_error(&source_relative, "read file", error))?;
            let target = destination.join(&name);
            fs::write(&target, &bytes)
                .map_err(|error| mount_copy_error(&source_relative, "write file", error))?;
            copy_modified_time(&entry.path(), &target, &source_relative)?;
            digest.update(b"file\0");
            digest.update(relative.to_string_lossy().as_bytes());
            digest.update(b"\0");
            digest.update(&bytes);
            files += 1;
        } else {
            return Err(RepositoryError::MountSource(format!(
                "source data contains an unsupported entry at {}",
                source_relative.display()
            )));
        }
    }
    Ok((hex::encode(digest.finalize()), files))
}

/// Probes the actual staging filesystem rather than inferring case semantics
/// from an operating-system or filesystem name.
fn destination_is_case_sensitive(destination: &Path) -> Result<bool, RepositoryError> {
    let lower = destination.join(".kero-case-probe");
    let upper = destination.join(".KERO-CASE-PROBE");
    fs::write(&lower, [])?;
    let case_sensitive = !upper.exists();
    fs::remove_file(lower)?;
    Ok(case_sensitive)
}

fn validate_destination_name(
    name: &std::ffi::OsStr,
    relative: &Path,
) -> Result<(), RepositoryError> {
    #[cfg(windows)]
    {
        let name = name.to_string_lossy();
        let trimmed = name.trim_end_matches([' ', '.']);
        let stem = trimmed
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_uppercase();
        let reserved = matches!(
            stem.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        );
        if trimmed != name
            || trimmed.is_empty()
            || reserved
            || name.chars().any(|character| {
                character.is_control()
                    || matches!(
                        character,
                        '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                    )
            })
        {
            return Err(RepositoryError::MountSource(format!(
                "source data name is incompatible with the destination filesystem at {}",
                relative.display()
            )));
        }
    }
    #[cfg(not(windows))]
    let _ = (name, relative);
    Ok(())
}

/// Preserves the portable timestamp supported by ordinary filesystems.
fn copy_modified_time(
    source: &Path,
    destination: &Path,
    relative: &Path,
) -> Result<(), RepositoryError> {
    let modified = fs::metadata(source)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| mount_copy_error(relative, "read modified timestamp", error))?;
    filetime::set_file_mtime(destination, filetime::FileTime::from_system_time(modified))
        .map_err(|error| mount_copy_error(relative, "preserve modified timestamp", error))
}

fn mount_copy_error(relative: &Path, operation: &str, error: std::io::Error) -> RepositoryError {
    let path = if relative.as_os_str().is_empty() {
        ".".into()
    } else {
        relative.display().to_string()
    };
    RepositoryError::MountSource(format!("{operation} at {path}: {error}"))
}

/// Applies the host-visible read-only boundary to a copied mount tree.
///
/// The service still treats mount writes as denied even if a user changes file
/// permissions outside KERO. This filesystem setting prevents ordinary editors
/// from mistaking a materialized copy for editable local knowledge.
fn set_tree_readonly(path: &Path, readonly: bool) -> Result<(), RepositoryError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(RepositoryError::MountSource(format!(
            "mount contains an unsupported symbolic link: {}",
            path.display()
        )));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            set_tree_readonly(&entry?.path(), readonly)?;
        }
    }
    let mut permissions = metadata.permissions();
    permissions.set_readonly(readonly);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn valid_mount_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn resolve_under(base: &Path, relative: &Path, scope: &str) -> Result<PathBuf, RepositoryError> {
    if relative.is_absolute()
        || relative.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(RepositoryError::PathEscape {
            scope: scope.into(),
            path: relative.into(),
        });
    }
    Ok(base.join(relative))
}

/// Resolves and updates the actual Git exclude file, including linked worktrees.
pub fn install_git_excludes(boundary: &RepositoryBoundary) -> Result<(), RepositoryError> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(&boundary.root)
        .args(["rev-parse", "--git-path", "info/exclude"])
        .output()?;
    if !output.status.success() {
        return Ok(());
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if raw.is_empty() {
        return Err(RepositoryError::Git(
            "Git returned an empty exclude path".into(),
        ));
    }
    let path = PathBuf::from(raw);
    let path = if path.is_absolute() {
        path
    } else {
        boundary.root.join(path)
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut additions = String::new();
    for directory in OWNED_DIRECTORIES {
        let entry = format!("/.kero/{directory}/");
        if !existing.lines().any(|line| line == entry) {
            additions.push_str(&entry);
            additions.push('\n');
        }
    }
    if !additions.is_empty() {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(additions.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod filesystem_compatibility_tests {
    use super::validate_destination_name;
    use std::ffi::OsStr;
    use std::path::Path;

    #[cfg(windows)]
    #[test]
    fn rejects_windows_reserved_destination_names() {
        for name in ["CON", "aux.txt", "LPT9.", "trailing. "] {
            assert!(
                validate_destination_name(OsStr::new(name), Path::new(name)).is_err(),
                "{name} should not be materialized on Windows"
            );
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn leaves_native_destination_names_to_the_host_filesystem() {
        assert!(
            validate_destination_name(OsStr::new("ordinary:name"), Path::new("ordinary:name"))
                .is_ok()
        );
    }
}
