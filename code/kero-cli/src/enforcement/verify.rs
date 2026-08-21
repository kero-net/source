//! Layer 3 artifact admission.
//!
//! Boundaries verify this immutable authorization artifact and their own exact
//! binding; this module never re-runs policy resolution.

use crate::artifact::{ArtifactBinding, ArtifactError, ArtifactVerifier, VerifiedArtifact};
use std::path::Path;

pub fn verify_attempt(
    verifier: &ArtifactVerifier,
    artifact_path: &Path,
    binding: &ArtifactBinding,
) -> Result<VerifiedArtifact, ArtifactError> {
    verifier.verify_bound_path(artifact_path, binding)
}
