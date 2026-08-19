//! The first concrete Linux Layer 3 adapter: an exact, atomic workspace write.

use crate::artifact::ArtifactVerifier;
use crate::canonical;
use crate::result::{
    Authorization, Capability, Enforcement, Execution, ExecutionResult, Verification,
};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
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
    #[error("workspace.replay")]
    Replay,
    #[error("workspace.write: {0}")]
    Write(String),
}

/// Broker for one configured workspace root. The caller supplies only bytes and
/// an already verified artifact; policy is never evaluated here.
pub struct WorkspaceWriteBroker {
    root: PathBuf,
    nonce_dir: PathBuf,
    audit: PathBuf,
}

impl WorkspaceWriteBroker {
    pub fn new(
        root: impl Into<PathBuf>,
        state: impl Into<PathBuf>,
    ) -> Result<Self, WorkspaceWriteError> {
        let root = root.into();
        if !root.is_dir() {
            return Err(WorkspaceWriteError::Root(std::io::Error::from(
                std::io::ErrorKind::NotFound,
            )));
        }
        let state = state.into();
        fs::create_dir_all(&state)?;
        Ok(Self {
            root,
            nonce_dir: state.join("nonces"),
            audit: state.join("audit.log"),
        })
    }

    pub fn write(
        &self,
        verifier: &ArtifactVerifier,
        artifact: &Path,
        content: &[u8],
    ) -> Result<ExecutionResult, WorkspaceWriteError> {
        let verified = verifier
            .verify_path(artifact)
            .map_err(|e| WorkspaceWriteError::Write(e.to_string()))?;
        let a = verified.artifact();
        let target = a
            .targets
            .first()
            .ok_or(WorkspaceWriteError::TargetMismatch)?;
        let relative = target
            .strip_prefix("path:")
            .ok_or(WorkspaceWriteError::Target)?;
        let relative = validate_relative(relative)?;
        let destination = self.root.join(relative);
        let parent = destination.parent().ok_or(WorkspaceWriteError::Target)?;
        let root = fs::canonicalize(&self.root)?;
        let parent_real = fs::canonicalize(parent).map_err(|_| WorkspaceWriteError::Target)?;
        if !parent_real.starts_with(&root)
            || destination.exists() && fs::symlink_metadata(&destination)?.file_type().is_symlink()
        {
            return Err(WorkspaceWriteError::Target);
        }
        fs::create_dir_all(&self.nonce_dir)?;
        let nonce = self.nonce_dir.join(&a.nonce);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&nonce)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    WorkspaceWriteError::Replay
                } else {
                    e.into()
                }
            })?;
        self.audit_event("prepared", &a.artifact_id, target)?;
        let tmp = parent.join(format!(".scope-write-{}", a.nonce));
        let result = (|| {
            let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
            file.write_all(content)?;
            file.sync_all()?;
            fs::rename(&tmp, &destination)?;
            let dir = OpenOptions::new().read(true).open(parent)?;
            dir.sync_all()?;
            Ok::<(), std::io::Error>(())
        })();
        let _ = fs::remove_file(&tmp);
        match result {
            Ok(()) => {
                self.audit_event("completed", &a.artifact_id, target)?;
                Ok(ExecutionResult {
                    schema: "scope/execution-result/v1".into(),
                    authorization: Authorization::Allow,
                    verification: Verification::Valid,
                    capability: Capability::Can,
                    enforcement: Enforcement::Advisory,
                    execution: Execution::Execute,
                    reason: "workspace.write.completed".into(),
                })
            }
            Err(e) => {
                let _ = self.audit_event("failed", &a.artifact_id, target);
                Err(WorkspaceWriteError::Write(e.to_string()))
            }
        }
    }

    fn audit_event(
        &self,
        phase: &str,
        artifact: &str,
        target: &str,
    ) -> Result<(), WorkspaceWriteError> {
        let audit = fs::read_to_string(&self.audit).unwrap_or_default();
        let previous = audit.lines().last().unwrap_or("");
        let event = json!({"schema":"scope/workspace-audit/v1","phase":phase,"artifact_id":artifact,"target":target,"previous":canonical::sha256(previous.as_bytes())});
        let line = format!(
            "{}\n",
            String::from_utf8(
                canonical::canonicalize(&event)
                    .map_err(|e| WorkspaceWriteError::Write(e.to_string()))?
            )
            .unwrap()
        );
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.audit)?;
        file.write_all(line.as_bytes())?;
        file.sync_all()?;
        Ok(())
    }
}

fn validate_relative(value: &str) -> Result<PathBuf, WorkspaceWriteError> {
    if value.is_empty() || value.contains('\\') || value.as_bytes().contains(&0) {
        return Err(WorkspaceWriteError::Target);
    }
    let path = Path::new(value);
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(WorkspaceWriteError::Target);
        }
    }
    Ok(path.to_path_buf())
}
