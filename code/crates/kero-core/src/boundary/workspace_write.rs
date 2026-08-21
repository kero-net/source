//! Linux descriptor-relative exact workspace write boundary.

use crate::artifact::{ArtifactBinding, ArtifactVerifier};
use crate::canonical;
use crate::enforcement::{Attempt, AuditLog, BrokerState, Lifecycle, NonceStore, verify_attempt};
use crate::result::{Authorization, Capability, ExecutionResult, Verification};
use serde_json::json;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

pub const BOUNDARY: &str = "broker.workspace-write/v1";
pub const THREAT_MODEL: &str = "workspace-caller/v1";

#[derive(Debug, Error)]
pub enum WorkspaceWriteError {
    #[error("workspace.root-unavailable: {0}")]
    Root(#[from] std::io::Error),
    #[error("workspace.target-invalid")]
    Target,
    #[error("workspace.target-mismatch")]
    TargetMismatch,
    #[error("workspace.binding-mismatch")]
    Binding,
    #[error("workspace.replay")]
    Replay,
    #[error("workspace.audit: {0}")]
    Audit(String),
    #[error("workspace.write: {0}")]
    Write(String),
}

pub struct WorkspaceWriteBroker {
    root: File,
    state: BrokerState,
}

impl WorkspaceWriteBroker {
    pub fn new(
        root: impl Into<PathBuf>,
        state: impl Into<PathBuf>,
    ) -> Result<Self, WorkspaceWriteError> {
        let root = open_directory(&root.into())?;
        let state = BrokerState::open(state.into())
            .map_err(|error| WorkspaceWriteError::Write(error.to_string()))?;
        Ok(Self { root, state })
    }

    pub fn write(
        &self,
        verifier: &ArtifactVerifier,
        artifact: &Path,
        binding: &ArtifactBinding,
        content: &[u8],
    ) -> Result<ExecutionResult, WorkspaceWriteError> {
        let verified = verify_attempt(verifier, artifact, binding)
            .map_err(|error| WorkspaceWriteError::Write(error.to_string()))?;
        let artifact = verified.artifact();
        let [target] = artifact.targets.as_slice() else {
            return Err(WorkspaceWriteError::TargetMismatch);
        };
        let relative = target
            .strip_prefix("path:")
            .ok_or(WorkspaceWriteError::Target)?;
        let parts = relative_parts(relative)?;
        let expected = json!({"path": target, "content_digest": canonical::sha256(content)});
        if artifact.execution_binding != expected {
            return Err(WorkspaceWriteError::Binding);
        }
        let generation = artifact
            .boundary_generation
            .ok_or(WorkspaceWriteError::Binding)?;
        self.state
            .require_active(generation)
            .map_err(|error| WorkspaceWriteError::Write(error.to_string()))?;
        let attempt = Attempt {
            artifact_id: artifact.artifact_id.clone(),
            nonce: artifact.nonce.clone(),
            boundary: BOUNDARY.into(),
            operation: artifact.operation.clone(),
            target_digest: artifact.target_digest.clone(),
            snapshot_digest: artifact.snapshot.digest.clone(),
            generation,
        };
        let nonces = NonceStore::new(self.state.clone());
        nonces.admit(&artifact.nonce, generation).map_err(|error| {
            if error.to_string() == "nonce.replay" {
                WorkspaceWriteError::Replay
            } else {
                WorkspaceWriteError::Write(error.to_string())
            }
        })?;
        let audit = AuditLog::new(self.state.audit_path());
        audit
            .append(attempt.clone(), Lifecycle::Prepared)
            .map_err(|error| WorkspaceWriteError::Audit(error.to_string()))?;
        match write_relative(&self.root, &parts, &artifact.nonce, content) {
            Ok(()) => {
                audit
                    .append(attempt, Lifecycle::Completed)
                    .map_err(|error| WorkspaceWriteError::Audit(error.to_string()))?;
                Ok(ExecutionResult::executed_advisory(
                    Authorization::Allow,
                    Verification::Valid,
                    Capability::Can,
                    "workspace.write.completed",
                ))
            }
            Err(error) => {
                let _ = audit.append(attempt, Lifecycle::Uncertain);
                Err(error)
            }
        }
    }

    pub fn generation(&self) -> Result<u64, WorkspaceWriteError> {
        self.state
            .generation()
            .map_err(|error| WorkspaceWriteError::Write(error.to_string()))
    }
}

fn relative_parts(value: &str) -> Result<Vec<CString>, WorkspaceWriteError> {
    if value.is_empty()
        || value.contains('\\')
        || value.contains("//")
        || value.as_bytes().contains(&0)
    {
        return Err(WorkspaceWriteError::Target);
    }
    let mut parts = Vec::new();
    for component in Path::new(value).components() {
        let Component::Normal(part) = component else {
            return Err(WorkspaceWriteError::Target);
        };
        parts.push(CString::new(part.as_encoded_bytes()).map_err(|_| WorkspaceWriteError::Target)?);
    }
    if parts.is_empty() {
        return Err(WorkspaceWriteError::Target);
    }
    Ok(parts)
}

fn open_directory(path: &Path) -> Result<File, std::io::Error> {
    let path = CString::new(path.as_os_str().as_encoded_bytes())
        .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
    // SAFETY: CString is NUL terminated and ownership of a successful fd is transferred to File.
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: fd was newly opened above.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn open_child_directory(parent: &File, name: &CString) -> Result<File, WorkspaceWriteError> {
    // SAFETY: parent is live and name is NUL terminated.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(WorkspaceWriteError::Target);
    }
    // SAFETY: fd was newly opened above.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn write_relative(
    root: &File,
    parts: &[CString],
    nonce: &str,
    content: &[u8],
) -> Result<(), WorkspaceWriteError> {
    let (filename, parents) = parts.split_last().ok_or(WorkspaceWriteError::Target)?;
    let mut parent = root.try_clone().map_err(WorkspaceWriteError::Root)?;
    for part in parents {
        parent = open_child_directory(&parent, part)?;
    }
    let temporary =
        CString::new(format!(".kero-write-{nonce}")).map_err(|_| WorkspaceWriteError::Target)?;
    // SAFETY: descriptor and names are valid; creation cannot follow a symlink.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            temporary.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return Err(WorkspaceWriteError::Write(
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: fd was newly opened above.
    let mut file = unsafe { File::from_raw_fd(fd) };
    let outcome = (|| -> Result<(), WorkspaceWriteError> {
        file.write_all(content).map_err(WorkspaceWriteError::Root)?;
        file.sync_all().map_err(WorkspaceWriteError::Root)?;
        // SAFETY: both names are relative to the same held directory descriptor.
        if unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                temporary.as_ptr(),
                parent.as_raw_fd(),
                filename.as_ptr(),
            )
        } != 0
        {
            return Err(WorkspaceWriteError::Write(
                std::io::Error::last_os_error().to_string(),
            ));
        }
        parent.sync_all().map_err(WorkspaceWriteError::Root)?;
        Ok(())
    })();
    drop(file);
    if outcome.is_err() {
        unsafe { libc::unlinkat(parent.as_raw_fd(), temporary.as_ptr(), 0) };
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_all_non_relative_single_target_forms() {
        for path in ["", "/absolute", ".", "..", "a/../b", "a\\b", "a//b"] {
            assert!(relative_parts(path).is_err(), "{path:?} must be rejected");
        }
        assert_eq!(relative_parts("dir/file").unwrap().len(), 2);
    }

    #[test]
    fn held_parent_descriptor_survives_path_replacement() {
        let temporary = tempdir().unwrap();
        let root = temporary.path().join("workspace");
        let outside = temporary.path().join("outside");
        std::fs::create_dir_all(root.join("safe")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let descriptor = open_directory(&root).unwrap();
        let parent = open_child_directory(&descriptor, &CString::new("safe").unwrap()).unwrap();
        std::fs::rename(root.join("safe"), root.join("replaced")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("safe")).unwrap();
        let filename = CString::new("file").unwrap();
        let nonce = CString::new(".kero-write-test").unwrap();
        // The held descriptor still names the original directory, not the
        // replacement symlink visible at the path.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                nonce.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW,
                0o600,
            )
        };
        assert!(fd >= 0);
        let mut file = unsafe { File::from_raw_fd(fd) };
        file.write_all(b"safe").unwrap();
        file.sync_all().unwrap();
        assert_eq!(
            unsafe {
                libc::renameat(
                    parent.as_raw_fd(),
                    nonce.as_ptr(),
                    parent.as_raw_fd(),
                    filename.as_ptr(),
                )
            },
            0
        );
        parent.sync_all().unwrap();
        assert_eq!(std::fs::read(root.join("replaced/file")).unwrap(), b"safe");
        assert!(!outside.join("file").exists());
    }
}
