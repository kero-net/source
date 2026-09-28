//! Narrow platform adapter contracts used by otherwise target-neutral core logic.
//!
//! The core never infers operating-system conventions from a target triple or
//! CPU architecture. Frontends and installers provide those facts explicitly
//! through these contracts.

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Whether applying an operation at a path requires elevated privileges.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrivilegeRequirement {
    None,
    Elevation,
}

/// Stable, non-secret description of a signing identity discovered by a
/// platform adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityCandidate {
    /// Opaque reference suitable for later user/global configuration.
    ///
    /// This must not contain private key material.
    pub reference: String,
    pub kind: IdentityKind,
}

/// Identity kinds that Release 1 platform adapters may report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityKind {
    OpenSshEd25519,
}

/// Errors raised by platform adapter operations.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum PlatformAdapterError {
    #[error("platform.adapter: {0}")]
    Operation(String),
}

/// Supplies platform path syntax and conventional KERO-home locations.
///
/// `is_absolute_path` exists here deliberately: absolute-path syntax is a
/// platform fact and must not be interpreted using the host running a test or
/// planning operation for another target.
pub trait PlatformPathProvider {
    fn user_home(&self) -> PathBuf;
    fn system_home(&self) -> Option<PathBuf>;
    fn is_absolute_path(&self, path: &Path) -> bool;
}

/// Supplies privilege requirements without teaching core policy about OS
/// conventions or protected filesystem locations.
pub trait PrivilegeProvider {
    fn privilege_requirement(
        &self,
        path: &Path,
    ) -> Result<PrivilegeRequirement, PlatformAdapterError>;
}

/// Setup planning needs both path and privilege facts, but owns neither.
pub trait SetupPlatform: PlatformPathProvider + PrivilegeProvider {}
impl<T> SetupPlatform for T where T: PlatformPathProvider + PrivilegeProvider {}

/// Applies or removes the native hidden-file attribute where the platform has
/// one. A platform with no such attribute may implement this as a successful
/// no-op.
pub trait HiddenAttributeAdapter {
    fn set_hidden(&self, path: &Path, hidden: bool) -> Result<(), PlatformAdapterError>;
}

/// Discovers usable signing identities without exposing secret key material to
/// repository code.
pub trait IdentityDiscoveryAdapter {
    fn discover_signing_identities(&self) -> Result<Vec<IdentityCandidate>, PlatformAdapterError>;
}
