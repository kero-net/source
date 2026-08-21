//! Reusable fail-closed admission before a native boundary side effect.

use super::{
    Attempt, AuditLog, BrokerState, CapabilityObservation, CheckpointError, Lifecycle, NonceStore,
    verify_checkpoint,
};
use crate::result::Capability;
use chrono::{DateTime, Utc};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdmissionError {
    #[error("admission.capability-unavailable")]
    Capability,
    #[error("admission.checkpoint: {0}")]
    Checkpoint(#[from] CheckpointError),
    #[error("admission.audit: {0}")]
    Audit(#[from] super::AuditError),
    #[error("admission.nonce: {0}")]
    Nonce(#[from] super::nonce::NonceError),
    #[error("admission.state: {0}")]
    State(#[from] super::StateError),
}

/// Performs all shared evidence checks and durably consumes the nonce before
/// returning. A caller may perform its side effect only after this succeeds.
pub fn admit(
    state: &BrokerState,
    attempt: Attempt,
    capability: &CapabilityObservation,
    now: DateTime<Utc>,
    checkpoint_path: &Path,
    checkpoint_key: &[u8],
) -> Result<(), AdmissionError> {
    if capability.capability != Capability::Can || !capability.is_current(now) {
        return Err(AdmissionError::Capability);
    }
    let checkpoint = verify_checkpoint(checkpoint_path, checkpoint_key)?;
    if checkpoint.generation != attempt.generation || checkpoint.boundary != attempt.boundary {
        return Err(AdmissionError::Checkpoint(CheckpointError::Boundary));
    }
    state.require_active(attempt.generation)?;
    let audit = AuditLog::new(state.audit_path());
    if audit.verify()? != checkpoint.audit_head {
        return Err(AdmissionError::Checkpoint(CheckpointError::Audit));
    }
    NonceStore::new(state.clone()).admit(&attempt.nonce, attempt.generation)?;
    audit.append(attempt, Lifecycle::Prepared)?;
    Ok(())
}
