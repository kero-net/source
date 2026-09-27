//! First-run policy planning independent of terminal or graphical frontends.

use crate::platform::{PrivilegeRequirement, SetupPlatform};
use std::path::PathBuf;
use thiserror::Error;

/// Where KERO keeps user-level settings and identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HomeLocation {
    UserLocal,
    SystemWide,
    Custom(PathBuf),
    None,
}
/// Repository enrollment behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Enrollment {
    Ask,
    Automatic,
    Manual,
}
/// Whether setup may use an identity now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Identity {
    Reuse,
    ConfigureLater,
}
/// Trust-verification behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verification {
    Automatic,
    Warn,
    Strict,
    Permissive,
}
/// A validated first-run policy. It contains no private key material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupPolicy {
    pub home: HomeLocation,
    pub enrollment: Enrollment,
    pub identity: Identity,
    pub verification: Verification,
    pub shell_integration: bool,
    pub update_checks: bool,
    pub diagnostics: bool,
}
/// A side-effect-free description of setup work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupPlan {
    pub policy: SetupPolicy,
    /// Resolved by the frontend's platform adapter, never inferred by core.
    pub home_path: Option<PathBuf>,
    pub operations: Vec<String>,
    pub needs_elevation: bool,
}

/// Native or embedded host capability for creating the selected KERO home.
///
/// The core plans a home but never opens a host filesystem directly.
pub trait KeroHomeHost {
    /// Provisions the complete KERO home boundary and records only global
    /// preferences derived from `policy`.
    fn create_kero_home(
        &self,
        path: &std::path::Path,
        policy: &SetupPolicy,
    ) -> Result<(), SetupError>;
}
#[derive(Debug, Error)]
pub enum SetupError {
    #[error("setup.home-invalid: custom KERO home must be absolute for the selected platform")]
    CustomHomeRelative,
    #[error("setup.system-unsupported: system-wide KERO home is unsupported on this platform")]
    SystemUnsupported,
    #[error("setup.home-unrecognized: {0} exists but is not a marked KERO home")]
    HomeUnrecognized(PathBuf),
    #[error(transparent)]
    Platform(#[from] crate::platform::PlatformAdapterError),
    #[error("setup.io: {0}")]
    Io(#[from] std::io::Error),
}

impl Default for SetupPolicy {
    fn default() -> Self {
        Self {
            home: HomeLocation::UserLocal,
            enrollment: Enrollment::Ask,
            identity: Identity::Reuse,
            verification: Verification::Automatic,
            shell_integration: false,
            update_checks: true,
            diagnostics: true,
        }
    }
}

/// Computes setup operations without mutating a machine.
///
/// Path syntax and privilege requirements are supplied by `platform`; this
/// function contains no operating-system or CPU-specific policy.
pub fn plan<P>(policy: SetupPolicy, platform: &P) -> Result<SetupPlan, SetupError>
where
    P: SetupPlatform + ?Sized,
{
    let home = match &policy.home {
        HomeLocation::UserLocal => platform.user_home(),
        HomeLocation::SystemWide => platform
            .system_home()
            .ok_or(SetupError::SystemUnsupported)?,
        HomeLocation::Custom(path) if platform.is_absolute_path(path) => path.clone(),
        HomeLocation::Custom(_) => return Err(SetupError::CustomHomeRelative),
        HomeLocation::None => {
            return Ok(SetupPlan {
                policy,
                home_path: None,
                operations: vec!["do not create a KERO home".into()],
                needs_elevation: false,
            });
        }
    };

    let needs_elevation = matches!(
        platform.privilege_requirement(&home)?,
        PrivilegeRequirement::Elevation
    );

    Ok(SetupPlan {
        policy,
        home_path: Some(home.clone()),
        operations: vec![
            format!("create KERO home at {}", home.display()),
            "write user-level setup policy without private key material".into(),
        ],
        needs_elevation,
    })
}
/// Applies only the planned KERO home through a host capability.
/// Callers choose dry-run by not calling this function.
pub fn apply<H: KeroHomeHost>(plan: &SetupPlan, host: &H) -> Result<(), SetupError> {
    if let Some(home) = &plan.home_path {
        host.create_kero_home(home, &plan.policy)?;
    }
    Ok(())
}
