//! Environment lifecycle semantics implemented through host capabilities.

use crate::config;
use crate::host::{DataScope, LifecycleHost, ScopedPath};
use thiserror::Error;

/// KERO environment lifecycle failure.
#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("lifecycle.host: {0}")]
    Host(String),
    #[error("lifecycle.not-found: no KERO environment was found by the host")]
    NotFound,
}

fn host_error<E: std::error::Error>(error: E) -> LifecycleError {
    LifecycleError::Host(error.to_string())
}

/// Initializes a selected environment idempotently through its host.
///
/// A pre-existing boundary is accepted unchanged. A new boundary gets only
/// `config`, `data`, and `mnt`; runtime state remains absent.
pub fn initialize<H: LifecycleHost>(host: &mut H) -> Result<(), LifecycleError> {
    if host.environment_exists().map_err(host_error)? {
        return Ok(());
    }
    host.create_directory(&ScopedPath::new(DataScope::Local, Vec::new()).unwrap())
        .map_err(host_error)?;
    host.create_directory(&ScopedPath::new(DataScope::Mounts, Vec::new()).unwrap())
        .map_err(host_error)?;
    host.write(
        &ScopedPath::new(DataScope::Config, Vec::new()).unwrap(),
        config::default_config().as_bytes(),
    )
    .map_err(host_error)
}

/// Checks nearest-boundary discovery without interpreting native paths.
pub fn discover<H: LifecycleHost>(host: &mut H) -> Result<(), LifecycleError> {
    if host.discover_nearest_boundary().map_err(host_error)? {
        Ok(())
    } else {
        Err(LifecycleError::NotFound)
    }
}

/// Creates hidden runtime state only when an operation explicitly requires it.
pub fn ensure_runtime<H: LifecycleHost>(host: &mut H) -> Result<(), LifecycleError> {
    host.create_directory(&ScopedPath::new(DataScope::Runtime, Vec::new()).unwrap())
        .map_err(host_error)?;
    host.set_runtime_hidden(true).map_err(host_error)
}

/// Removes only disposable runtime state. A later `ensure_runtime` recreates it.
pub fn reset_runtime<H: LifecycleHost>(host: &mut H) -> Result<(), LifecycleError> {
    host.remove_runtime().map_err(host_error)
}
