use super::{AuditError, AuditLog, BrokerState, StateError};
use crate::canonical;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Checkpoint {
    pub schema: String,
    pub generation: u64,
    pub audit_head: String,
    pub snapshot_digest: String,
    pub boundary: String,
    pub signature: String,
}

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("checkpoint.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("checkpoint.malformed")]
    Malformed,
    #[error("checkpoint.signature-invalid")]
    Signature,
    #[error("checkpoint.state: {0}")]
    State(#[from] StateError),
    #[error("checkpoint.audit-mismatch")]
    Audit,
    #[error("checkpoint.snapshot-mismatch")]
    Snapshot,
    #[error("checkpoint.boundary-mismatch")]
    Boundary,
    #[error("checkpoint.audit: {0}")]
    AuditLog(#[from] AuditError),
    #[error(transparent)]
    Canonical(#[from] canonical::CanonicalError),
}

fn payload(checkpoint: &Checkpoint) -> Result<Vec<u8>, CheckpointError> {
    let mut value = serde_json::to_value(checkpoint).map_err(|_| CheckpointError::Malformed)?;
    value
        .as_object_mut()
        .ok_or(CheckpointError::Malformed)?
        .remove("signature");
    Ok(canonical::canonicalize(&value)?)
}

pub fn write_checkpoint(
    path: &Path,
    mut checkpoint: Checkpoint,
    key: &[u8],
) -> Result<(), CheckpointError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| CheckpointError::Signature)?;
    mac.update(&payload(&checkpoint)?);
    checkpoint.signature = format!("hmac-sha256:{}", hex::encode(mac.finalize().into_bytes()));
    let bytes = canonical::canonicalize(&checkpoint)?;
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn verify_checkpoint(path: &Path, key: &[u8]) -> Result<Checkpoint, CheckpointError> {
    let bytes = fs::read(path)?;
    let checkpoint: Checkpoint =
        serde_json::from_slice(&bytes).map_err(|_| CheckpointError::Malformed)?;
    if checkpoint.schema != "kero/checkpoint/v1" {
        return Err(CheckpointError::Malformed);
    }
    if canonical::canonicalize(&checkpoint)? != bytes {
        return Err(CheckpointError::Malformed);
    }
    let signature = checkpoint
        .signature
        .strip_prefix("hmac-sha256:")
        .and_then(|value| hex::decode(value).ok())
        .ok_or(CheckpointError::Signature)?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| CheckpointError::Signature)?;
    mac.update(&payload(&checkpoint)?);
    mac.verify_slice(&signature)
        .map_err(|_| CheckpointError::Signature)?;
    Ok(checkpoint)
}

/// Re-enables a deliberately safe-disabled broker only after independently
/// checking the signed checkpoint against the current durable audit state.
/// The generation must match exactly, so recovery cannot rewind it.
pub fn recover_state(
    state: &BrokerState,
    checkpoint: &Checkpoint,
    audit: &AuditLog,
    snapshot_digest: &str,
    boundary: &str,
) -> Result<(), CheckpointError> {
    if checkpoint.snapshot_digest != snapshot_digest {
        return Err(CheckpointError::Snapshot);
    }
    if checkpoint.boundary != boundary {
        return Err(CheckpointError::Boundary);
    }
    if audit.verify()? != checkpoint.audit_head {
        return Err(CheckpointError::Audit);
    }
    state.enable_after_recovery(checkpoint.generation)?;
    Ok(())
}
