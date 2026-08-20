//! Exact no-network Bubblewrap command boundary.

use crate::result::Capability;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

pub const BOUNDARY: &str = "sandbox.command/bwrap-v1";

#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub argv: Vec<String>,
    pub working_directory: String,
    pub environment: Vec<(String, String)>,
    pub timeout: Duration,
    pub output_limit: usize,
}

#[derive(Debug, Error)]
pub enum BubblewrapError {
    #[error("sandbox.command-invalid")]
    Invalid,
    #[error("sandbox.capability-unavailable")]
    Unavailable,
    #[error("sandbox.timeout")]
    Timeout,
    #[error("sandbox.output-overflow")]
    Overflow,
    #[error("sandbox.execution: {0}")]
    Execution(#[from] std::io::Error),
}

pub fn capability(binary: &Path) -> Capability {
    if binary.is_file() {
        Capability::Can
    } else {
        Capability::Cannot
    }
}

pub fn run(binary: impl Into<PathBuf>, spec: &CommandSpec) -> Result<Vec<u8>, BubblewrapError> {
    validate(spec)?;
    let binary = binary.into();
    if capability(&binary) != Capability::Can {
        return Err(BubblewrapError::Unavailable);
    }
    let mut command = Command::new(binary);
    command
        .args([
            "--die-with-parent",
            "--new-session",
            "--unshare-all",
            "--unshare-net",
        ])
        .args(["--ro-bind", "/", "/", "--tmpfs", "/tmp", "--clearenv"])
        .args(["--chdir", &spec.working_directory]);
    for (key, value) in &spec.environment {
        command.args(["--setenv", key, value]);
    }
    command
        .args(&spec.argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn()?;
    let mut stdout =
        child
            .stdout
            .take()
            .ok_or(BubblewrapError::Execution(std::io::Error::other(
                "sandbox stdout unavailable",
            )))?;
    let output_limit = spec.output_limit;
    let reader = std::thread::spawn(move || -> Result<(Vec<u8>, bool), std::io::Error> {
        let mut bytes = Vec::with_capacity(output_limit.min(8192));
        let mut overflow = false;
        let mut buffer = [0_u8; 8192];
        loop {
            let read = stdout.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let available = output_limit.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..read.min(available)]);
            overflow |= read > available;
        }
        Ok((bytes, overflow))
    });
    let deadline = Instant::now() + spec.timeout;
    loop {
        if child.try_wait()?.is_some() {
            let (bytes, overflow) = reader.join().map_err(|_| {
                BubblewrapError::Execution(std::io::Error::other("sandbox output reader panicked"))
            })??;
            if overflow {
                return Err(BubblewrapError::Overflow);
            }
            return Ok(bytes);
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let _ = child.wait();
            let _ = reader.join();
            return Err(BubblewrapError::Timeout);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn validate(spec: &CommandSpec) -> Result<(), BubblewrapError> {
    if spec.argv.is_empty()
        || spec.timeout.is_zero()
        || spec.output_limit == 0
        || !spec.working_directory.starts_with('/')
    {
        return Err(BubblewrapError::Invalid);
    }
    if spec
        .argv
        .iter()
        .any(|value| value.is_empty() || value.contains('\0'))
    {
        return Err(BubblewrapError::Invalid);
    }
    if spec.environment.iter().any(|(key, value)| {
        key.is_empty()
            || key.contains('\0')
            || value.contains('\0')
            || key.contains("TOKEN")
            || key.contains("SECRET")
            || key.contains("PASSWORD")
    }) {
        return Err(BubblewrapError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_or_unbounded_specs() {
        let spec = CommandSpec {
            argv: vec!["/bin/echo".into()],
            working_directory: "/".into(),
            environment: vec![("API_TOKEN".into(), "x".into())],
            timeout: Duration::from_secs(1),
            output_limit: 64,
        };
        assert!(matches!(
            run("/missing-bwrap", &spec),
            Err(BubblewrapError::Invalid)
        ));
    }

    #[test]
    fn runs_a_no_network_smoke_command_when_bubblewrap_is_available() {
        let binary = Path::new("/usr/bin/bwrap");
        if !binary.is_file() {
            return;
        }
        let spec = CommandSpec {
            argv: vec!["/bin/echo".into(), "scope-bwrap-smoke".into()],
            working_directory: "/".into(),
            environment: vec![],
            timeout: Duration::from_secs(5),
            output_limit: 128,
        };
        assert_eq!(run(binary, &spec).unwrap(), b"scope-bwrap-smoke\n");
    }
}
