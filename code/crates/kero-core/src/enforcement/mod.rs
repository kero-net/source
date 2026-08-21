//! Durable Layer 3 primitives shared by every native boundary.

mod admission;
mod audit;
mod capability;
mod deployment;
mod model;
mod nonce;
mod recovery;
mod state;
mod verify;

pub use admission::{AdmissionError, admit};
pub use audit::{AuditError, AuditEvent, AuditLog};
pub use capability::{CapabilityObservation, observe_command};
pub use deployment::{
    DeploymentAttestation, DeploymentError, DeploymentExpectation, sign_deployment,
    verify_deployment,
};
pub use model::{Attempt, Lifecycle};
pub use nonce::NonceStore;
pub use recovery::{
    Checkpoint, CheckpointError, recover_state, verify_checkpoint, write_checkpoint,
};
pub use state::{BrokerState, StateError};
pub use verify::verify_attempt;
