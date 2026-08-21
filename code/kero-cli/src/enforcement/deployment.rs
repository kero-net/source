//! Authenticated deployment evidence required for an `ENFORCED` claim.

use crate::canonical;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentAttestation {
    pub schema: String,
    pub boundary: String,
    pub threat_model: String,
    pub protected_paths: Vec<String>,
    pub mechanisms: Vec<String>,
    pub no_alternate_writers_or_executors: bool,
    pub generation: u64,
    pub issued_at: String,
    pub expires_at: String,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentExpectation {
    pub boundary: String,
    pub threat_model: String,
    pub protected_paths: Vec<String>,
    pub mechanisms: Vec<String>,
    pub generation: u64,
}

#[derive(Debug, Error)]
pub enum DeploymentError {
    #[error("deployment.malformed")]
    Malformed,
    #[error("deployment.schema-unsupported")]
    Schema,
    #[error("deployment.signature-invalid")]
    Signature,
    #[error("deployment.expired")]
    Expired,
    #[error("deployment.binding-mismatch")]
    Binding,
    #[error(transparent)]
    Canonical(#[from] canonical::CanonicalError),
}

fn payload(attestation: &DeploymentAttestation) -> Result<Vec<u8>, DeploymentError> {
    let mut value = serde_json::to_value(attestation).map_err(|_| DeploymentError::Malformed)?;
    value
        .as_object_mut()
        .ok_or(DeploymentError::Malformed)?
        .remove("signature");
    Ok(canonical::canonicalize(&value)?)
}

pub fn sign_deployment(
    mut attestation: DeploymentAttestation,
    key: &[u8],
) -> Result<DeploymentAttestation, DeploymentError> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| DeploymentError::Signature)?;
    mac.update(&payload(&attestation)?);
    attestation.signature = format!("hmac-sha256:{}", hex::encode(mac.finalize().into_bytes()));
    Ok(attestation)
}

/// Returns success only when the exact current boundary deployment is proven.
pub fn verify_deployment(
    attestation: &DeploymentAttestation,
    key: &[u8],
    expected: &DeploymentExpectation,
    now: DateTime<Utc>,
) -> Result<(), DeploymentError> {
    if attestation.schema != "kero/deployment-attestation/v1" {
        return Err(DeploymentError::Schema);
    }
    let signature = attestation
        .signature
        .strip_prefix("hmac-sha256:")
        .and_then(|value| hex::decode(value).ok())
        .ok_or(DeploymentError::Signature)?;
    let mut mac = HmacSha256::new_from_slice(key).map_err(|_| DeploymentError::Signature)?;
    mac.update(&payload(attestation)?);
    mac.verify_slice(&signature)
        .map_err(|_| DeploymentError::Signature)?;
    let issued = DateTime::parse_from_rfc3339(&attestation.issued_at)
        .map_err(|_| DeploymentError::Malformed)?
        .with_timezone(&Utc);
    let expires = DateTime::parse_from_rfc3339(&attestation.expires_at)
        .map_err(|_| DeploymentError::Malformed)?
        .with_timezone(&Utc);
    if now < issued - Duration::seconds(5) || now > expires {
        return Err(DeploymentError::Expired);
    }
    if attestation.boundary != expected.boundary
        || attestation.threat_model != expected.threat_model
        || attestation.protected_paths != expected.protected_paths
        || attestation.mechanisms != expected.mechanisms
        || !attestation.no_alternate_writers_or_executors
        || attestation.generation != expected.generation
    {
        return Err(DeploymentError::Binding);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::SecondsFormat;

    #[test]
    fn exact_current_attestation_is_required() {
        let now = Utc::now();
        let key = [4_u8; 32];
        let paths = vec!["/broker/state".into()];
        let mechanisms = vec!["openat-no-follow".into()];
        let attestation = sign_deployment(
            DeploymentAttestation {
                schema: "kero/deployment-attestation/v1".into(),
                boundary: "broker.test/v1".into(),
                threat_model: "test/v1".into(),
                protected_paths: paths.clone(),
                mechanisms: mechanisms.clone(),
                no_alternate_writers_or_executors: true,
                generation: 3,
                issued_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
                expires_at: (now + Duration::minutes(1)).to_rfc3339_opts(SecondsFormat::Secs, true),
                signature: String::new(),
            },
            &key,
        )
        .unwrap();
        let expectation = DeploymentExpectation {
            boundary: "broker.test/v1".into(),
            threat_model: "test/v1".into(),
            protected_paths: paths,
            mechanisms,
            generation: 3,
        };
        verify_deployment(&attestation, &key, &expectation, now).unwrap();
        let mismatched = DeploymentExpectation {
            generation: 4,
            ..expectation
        };
        assert!(verify_deployment(&attestation, &key, &mismatched, now).is_err());
    }
}
