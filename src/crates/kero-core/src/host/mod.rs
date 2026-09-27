//! Capability-scoped boundary between the portable core and a native host.
//!
//! The core describes repository-relative operations only. A host decides how
//! to perform filesystem, process, network, and presentation work for its own
//! operating system without leaking host paths or CPU facts into KERO data.

use std::fmt;

/// Native bootstrap adapter kept outside the WASM core.
///
/// It exists only for the current command-line host. Pass 3 replaces its
/// direct effects with calls through `KeroHost`.
#[cfg(not(target_arch = "wasm32"))]
#[path = "../knowledge/repository.rs"]
pub mod native_repository;

/// Native adapter for explicit, deterministic local knowledge snapshots.
#[cfg(not(target_arch = "wasm32"))]
#[path = "../knowledge/input.rs"]
pub mod native_input;

#[cfg(not(target_arch = "wasm32"))]
#[path = "../knowledge/processing.rs"]
pub mod native_processing;

/// The only data roots an operation may address.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataScope {
    /// The human-owned `.kero/config` document.
    Config,
    /// Repository-owned data beneath `.kero/data/`.
    Local,
    /// The `.kero/mnt/` root used only during environment initialization.
    Mounts,
    /// Data beneath one named `.kero/mnt/<name>/` root.
    Mount(String),
    /// Hidden, non-knowledge state beneath `.kero/.runtime/`.
    Runtime,
}

/// A validated relative path supplied to a host capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopedPath {
    scope: DataScope,
    segments: Vec<String>,
}

/// Host-boundary validation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HostPathError {
    EmptySegment,
    EscapingSegment(String),
    InvalidMountName(String),
}

impl fmt::Display for HostPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for HostPathError {}

impl ScopedPath {
    /// Constructs a capability path from portable, non-empty path segments.
    pub fn new(
        scope: DataScope,
        segments: impl IntoIterator<Item = String>,
    ) -> Result<Self, HostPathError> {
        if let DataScope::Mount(name) = &scope {
            if !valid_mount_name(name) {
                return Err(HostPathError::InvalidMountName(name.clone()));
            }
        }
        let segments = segments.into_iter().collect::<Vec<_>>();
        for segment in &segments {
            if segment.is_empty() {
                return Err(HostPathError::EmptySegment);
            }
            if segment == "." || segment == ".." || segment.contains(['/', '\\']) {
                return Err(HostPathError::EscapingSegment(segment.clone()));
            }
        }
        Ok(Self { scope, segments })
    }
    pub fn scope(&self) -> &DataScope {
        &self.scope
    }
    pub fn segments(&self) -> &[String] {
        &self.segments
    }
}

/// Effects a native host may explicitly grant to the portable core.
///
/// Every filesystem call is scoped. Process execution and networking are
/// intentionally named capabilities rather than ambient standard-library use.
pub trait KeroHost {
    type Error: std::error::Error + Send + Sync + 'static;

    fn read(&mut self, path: &ScopedPath) -> Result<Vec<u8>, Self::Error>;
    fn write(&mut self, path: &ScopedPath, bytes: &[u8]) -> Result<(), Self::Error>;
    fn create_directory(&mut self, path: &ScopedPath) -> Result<(), Self::Error>;
    fn materialize_mount(&mut self, mount: &str) -> Result<(), Self::Error>;
    fn execute(&mut self, capability: &str, input: &[u8]) -> Result<Vec<u8>, Self::Error>;
    fn network(&mut self, capability: &str, request: &[u8]) -> Result<Vec<u8>, Self::Error>;
    fn diagnostic(&mut self, code: &str, message: &str);
}

/// Lifecycle effects that a host may grant for one selected repository.
///
/// The host owns repository discovery and native path resolution. The core
/// only asks about a selected environment and scoped entries within it.
pub trait LifecycleHost: KeroHost {
    fn environment_exists(&mut self) -> Result<bool, Self::Error>;
    fn discover_nearest_boundary(&mut self) -> Result<bool, Self::Error>;
    fn set_runtime_hidden(&mut self, hidden: bool) -> Result<(), Self::Error>;
    fn remove_runtime(&mut self) -> Result<(), Self::Error>;
}

fn valid_mount_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
