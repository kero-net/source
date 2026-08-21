use crate::canonical;
use crate::policy::{AuthorizationRequest, PureAuthorizationResult};
use crate::result::Authorization;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

pub const ARTIFACT_SCHEMA: &str = "terminal-enforcement/authorization-artifact/v1";
pub const SNAPSHOT_SCHEMA: &str = "terminal-policy/snapshot/v1";
pub const RESOLVER_VERSION: &str = "terminal-policy-resolver/v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArtifactAuthentication {
    pub mechanism: String,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArtifactAuthorization {
    pub decision: Authorization,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SnapshotBinding {
    pub id: String,
    pub digest: String,
    pub schema: String,
    pub resolver_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthorizationArtifact {
    pub schema: String,
    pub artifact_id: String,
    pub authorization: ArtifactAuthorization,
    pub request_id: String,
    pub request_digest: String,
    pub operation: String,
    pub principal: String,
    pub session_id: String,
    pub targets: Vec<String>,
    pub target_digest: String,
    pub trusted_context_digest: String,
    pub snapshot: SnapshotBinding,
    pub issued_at: String,
    pub expires_at: String,
    pub nonce: String,
    pub replay_policy: String,
    pub audience: String,
    pub boundary_generation: Option<u64>,
    pub execution_binding: Value,
    pub execution_binding_digest: String,
    pub authentication: ArtifactAuthentication,
}

#[derive(Clone, Debug)]
pub struct ArtifactVerifier {
    expected_audience: String,
    expected_operation: String,
    key_path: PathBuf,
    snapshot_path: PathBuf,
}

/// The native boundary's expected identity and operation binding. This is
/// supplied by broker-owned configuration, never by a caller-provided
/// artifact field alone.
#[derive(Clone, Debug)]
pub struct ArtifactBinding {
    pub principal: String,
    pub session_id: String,
    pub targets: Vec<String>,
    pub trusted_context_digest: String,
    pub execution_binding: Value,
    pub boundary_generation: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct VerifiedArtifact {
    artifact: AuthorizationArtifact,
}

#[derive(Clone, Debug)]
pub struct ArtifactIssueOptions {
    pub ttl_seconds: u64,
    pub now: DateTime<Utc>,
    pub audience: String,
    pub execution_binding: Value,
    pub boundary_generation: Option<u64>,
}

impl VerifiedArtifact {
    pub fn artifact(&self) -> &AuthorizationArtifact {
        &self.artifact
    }
}

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("artifact.unavailable: {0}")]
    ArtifactUnavailable(std::io::Error),
    #[error("artifact.malformed: {0}")]
    ArtifactMalformed(serde_json::Error),
    #[error("key.unavailable: {0}")]
    KeyUnavailable(std::io::Error),
    #[error("key.permissions-not-private")]
    KeyPermissions,
    #[error("key.too-short")]
    KeyTooShort,
    #[error("artifact.schema-unsupported")]
    Schema,
    #[error("artifact.authentication-unsupported")]
    Authentication,
    #[error("artifact.signature-invalid")]
    Signature,
    #[error("artifact.audience-mismatch")]
    Audience,
    #[error("artifact.operation-unsupported")]
    Operation,
    #[error("artifact.authorization-denied")]
    AuthorizationDenied,
    #[error("artifact.replay-policy-unsupported")]
    ReplayPolicy,
    #[error("artifact.resolver-binding-mismatch")]
    ResolverBinding,
    #[error("snapshot.unavailable: {0}")]
    SnapshotUnavailable(std::io::Error),
    #[error("snapshot.digest-mismatch")]
    SnapshotDigest,
    #[error("artifact.time-malformed")]
    Time,
    #[error("artifact.not-yet-valid")]
    NotYetValid,
    #[error("artifact.expired")]
    Expired,
    #[error("artifact.target-digest-mismatch")]
    TargetDigest,
    #[error("artifact.execution-binding-mismatch")]
    ExecutionBinding,
    #[error(transparent)]
    Canonical(#[from] canonical::CanonicalError),
}

impl ArtifactVerifier {
    pub fn new(
        expected_audience: impl Into<String>,
        expected_operation: impl Into<String>,
        key_path: impl Into<PathBuf>,
        snapshot_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            expected_audience: expected_audience.into(),
            expected_operation: expected_operation.into(),
            key_path: key_path.into(),
            snapshot_path: snapshot_path.into(),
        }
    }

    pub fn verify_path(&self, artifact_path: &Path) -> Result<VerifiedArtifact, ArtifactError> {
        let bytes = fs::read(artifact_path).map_err(ArtifactError::ArtifactUnavailable)?;
        let artifact: AuthorizationArtifact =
            serde_json::from_slice(&bytes).map_err(ArtifactError::ArtifactMalformed)?;
        self.verify(artifact, Utc::now())
    }

    /// Reads an artifact and verifies all boundary-owned fields without
    /// invoking Layer 2 policy resolution.
    pub fn verify_bound_path(
        &self,
        artifact_path: &Path,
        expected: &ArtifactBinding,
    ) -> Result<VerifiedArtifact, ArtifactError> {
        let bytes = fs::read(artifact_path).map_err(ArtifactError::ArtifactUnavailable)?;
        let artifact: AuthorizationArtifact =
            serde_json::from_slice(&bytes).map_err(ArtifactError::ArtifactMalformed)?;
        self.verify_bound(artifact, Utc::now(), expected)
    }

    pub fn verify(
        &self,
        artifact: AuthorizationArtifact,
        now: DateTime<Utc>,
    ) -> Result<VerifiedArtifact, ArtifactError> {
        if artifact.schema != ARTIFACT_SCHEMA {
            return Err(ArtifactError::Schema);
        }
        if artifact.authentication.mechanism != "hmac-sha256/v1" {
            return Err(ArtifactError::Authentication);
        }
        let key = read_key(&self.key_path)?;
        let mut payload = serde_json::to_value(&artifact)?;
        payload
            .as_object_mut()
            .expect("artifact serializes as object")
            .remove("authentication");
        let supplied = artifact
            .authentication
            .signature
            .strip_prefix("hmac-sha256:")
            .and_then(|value| hex::decode(value).ok())
            .ok_or(ArtifactError::Signature)?;
        let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| ArtifactError::KeyTooShort)?;
        mac.update(&canonical::canonicalize(&payload)?);
        mac.verify_slice(&supplied)
            .map_err(|_| ArtifactError::Signature)?;
        if artifact.audience != self.expected_audience {
            return Err(ArtifactError::Audience);
        }
        if artifact.operation != self.expected_operation || artifact.targets.len() != 1 {
            return Err(ArtifactError::Operation);
        }
        if artifact.authorization.decision != Authorization::Allow {
            return Err(ArtifactError::AuthorizationDenied);
        }
        if artifact.replay_policy != "single-use" {
            return Err(ArtifactError::ReplayPolicy);
        }
        if artifact.snapshot.schema != SNAPSHOT_SCHEMA
            || artifact.snapshot.resolver_version != RESOLVER_VERSION
        {
            return Err(ArtifactError::ResolverBinding);
        }
        let snapshot = fs::read(&self.snapshot_path).map_err(ArtifactError::SnapshotUnavailable)?;
        if canonical::sha256(&snapshot) != artifact.snapshot.digest {
            return Err(ArtifactError::SnapshotDigest);
        }
        let issued =
            DateTime::parse_from_rfc3339(&artifact.issued_at).map_err(|_| ArtifactError::Time)?;
        let expires =
            DateTime::parse_from_rfc3339(&artifact.expires_at).map_err(|_| ArtifactError::Time)?;
        if now < issued.with_timezone(&Utc) - Duration::seconds(5) {
            return Err(ArtifactError::NotYetValid);
        }
        if now > expires {
            return Err(ArtifactError::Expired);
        }
        if canonical::digest(&artifact.targets)? != artifact.target_digest {
            return Err(ArtifactError::TargetDigest);
        }
        if canonical::digest(&artifact.execution_binding)? != artifact.execution_binding_digest {
            return Err(ArtifactError::ExecutionBinding);
        }
        Ok(VerifiedArtifact { artifact })
    }

    pub fn verify_bound(
        &self,
        artifact: AuthorizationArtifact,
        now: DateTime<Utc>,
        expected: &ArtifactBinding,
    ) -> Result<VerifiedArtifact, ArtifactError> {
        let verified = self.verify(artifact, now)?;
        let actual = verified.artifact();
        if actual.principal != expected.principal
            || actual.session_id != expected.session_id
            || actual.targets != expected.targets
            || actual.trusted_context_digest != expected.trusted_context_digest
            || actual.boundary_generation != expected.boundary_generation
            || actual.execution_binding != expected.execution_binding
        {
            return Err(ArtifactError::ExecutionBinding);
        }
        Ok(verified)
    }
}

pub fn issue_artifact(
    decision: &PureAuthorizationResult,
    request: &AuthorizationRequest,
    snapshot_path: &Path,
    key_path: &Path,
    options: ArtifactIssueOptions,
) -> Result<AuthorizationArtifact, ArtifactError> {
    if !(1..=3600).contains(&options.ttl_seconds)
        || decision.schema != "terminal-policy/authorization-result/v1"
        || request.schema != "terminal-policy/request/v1"
        || decision.decision != "ALLOW"
    {
        return Err(ArtifactError::AuthorizationDenied);
    }
    let targets: Vec<String> = request
        .targets
        .iter()
        .map(|item| item.canonical())
        .collect();
    if decision.request_id != request.request_id
        || decision.principal != request.principal
        || decision.operation != request.operation
        || decision.targets != targets
        || targets.len() != 1
    {
        return Err(ArtifactError::Operation);
    }
    let snapshot_bytes = fs::read(snapshot_path).map_err(ArtifactError::SnapshotUnavailable)?;
    if canonical::sha256(&snapshot_bytes) != decision.snapshot.digest {
        return Err(ArtifactError::SnapshotDigest);
    }
    let session_id = request
        .session
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(ArtifactError::Operation)?;
    let request_wire = serde_json::json!({
        "schema": request.schema,
        "request_id": request.request_id,
        "principal": request.principal,
        "operation": request.operation,
        "targets": targets,
        "session": request.session,
        "context": request.context,
    });
    let mut random = [0_u8; 32];
    getrandom::fill(&mut random).map_err(|_| ArtifactError::Authentication)?;
    let artifact_id = format!("artifact.{}", hex::encode(&random[..16]));
    let nonce = hex::encode(random);
    let mut artifact = AuthorizationArtifact {
        schema: ARTIFACT_SCHEMA.into(),
        artifact_id,
        authorization: ArtifactAuthorization {
            decision: Authorization::Allow,
            reason: decision.reason.clone(),
        },
        request_id: request.request_id.clone(),
        request_digest: canonical::digest(&request_wire)?,
        operation: request.operation.clone(),
        principal: request.principal.clone(),
        session_id: session_id.into(),
        targets: targets.clone(),
        target_digest: canonical::digest(&targets)?,
        trusted_context_digest: canonical::digest(&request.context)?,
        snapshot: SnapshotBinding {
            id: decision.snapshot.id.clone(),
            digest: decision.snapshot.digest.clone(),
            schema: decision.snapshot.schema.clone(),
            resolver_version: decision.snapshot.resolver_version.clone(),
        },
        issued_at: options.now.to_rfc3339_opts(SecondsFormat::Secs, true),
        expires_at: (options.now + Duration::seconds(options.ttl_seconds as i64))
            .to_rfc3339_opts(SecondsFormat::Secs, true),
        nonce,
        replay_policy: "single-use".into(),
        audience: options.audience,
        boundary_generation: options.boundary_generation,
        execution_binding_digest: canonical::digest(&options.execution_binding)?,
        execution_binding: options.execution_binding,
        authentication: ArtifactAuthentication {
            mechanism: "hmac-sha256/v1".into(),
            signature: String::new(),
        },
    };
    let key = read_key(key_path)?;
    let mut payload = serde_json::to_value(&artifact)?;
    payload
        .as_object_mut()
        .expect("artifact object")
        .remove("authentication");
    let mut mac = HmacSha256::new_from_slice(&key).map_err(|_| ArtifactError::KeyTooShort)?;
    mac.update(&canonical::canonicalize(&payload)?);
    artifact.authentication.signature =
        format!("hmac-sha256:{}", hex::encode(mac.finalize().into_bytes()));
    Ok(artifact)
}

fn read_key(path: &Path) -> Result<Vec<u8>, ArtifactError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).map_err(ArtifactError::KeyUnavailable)?;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(ArtifactError::KeyPermissions);
        }
    }
    let bytes = fs::read(path).map_err(ArtifactError::KeyUnavailable)?;
    let key = trim_ascii(&bytes);
    if key.len() < 32 {
        return Err(ArtifactError::KeyTooShort);
    }
    Ok(key.to_vec())
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

impl From<serde_json::Error> for ArtifactError {
    fn from(value: serde_json::Error) -> Self {
        ArtifactError::ArtifactMalformed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::Mac;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    const TEST_KEY: [u8; 32] = [0; 32];

    fn fixture(now: DateTime<Utc>) -> (tempfile::TempDir, ArtifactVerifier, AuthorizationArtifact) {
        let directory = tempdir().unwrap();
        let key_path = directory.path().join("key");
        fs::write(&key_path, TEST_KEY).unwrap();
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
        let snapshot_path = directory.path().join("snapshot.json");
        fs::write(
            &snapshot_path,
            b"{\"schema\":\"terminal-policy/snapshot/v1\"}\n",
        )
        .unwrap();
        let targets = vec!["path:src/main.rs".to_string()];
        let binding = json!({});
        let mut artifact = AuthorizationArtifact {
            schema: ARTIFACT_SCHEMA.into(),
            artifact_id: "artifact.test".into(),
            authorization: ArtifactAuthorization {
                decision: Authorization::Allow,
                reason: "allow.fixture".into(),
            },
            request_id: "request.test".into(),
            request_digest: "sha256:fixture".into(),
            operation: "filesystem.write".into(),
            principal: "principal.test".into(),
            session_id: "session.test".into(),
            targets: targets.clone(),
            target_digest: canonical::digest(&targets).unwrap(),
            trusted_context_digest: canonical::digest(&json!([])).unwrap(),
            snapshot: SnapshotBinding {
                id: "snapshot.test".into(),
                digest: canonical::sha256(&fs::read(&snapshot_path).unwrap()),
                schema: SNAPSHOT_SCHEMA.into(),
                resolver_version: RESOLVER_VERSION.into(),
            },
            issued_at: (now - Duration::seconds(1)).to_rfc3339_opts(SecondsFormat::Secs, true),
            expires_at: (now + Duration::minutes(5)).to_rfc3339_opts(SecondsFormat::Secs, true),
            nonce: "nonce.test".into(),
            replay_policy: "single-use".into(),
            audience: "broker.workspace-write/v1".into(),
            boundary_generation: None,
            execution_binding: binding.clone(),
            execution_binding_digest: canonical::digest(&binding).unwrap(),
            authentication: ArtifactAuthentication {
                mechanism: "hmac-sha256/v1".into(),
                signature: String::new(),
            },
        };
        let mut payload = serde_json::to_value(&artifact).unwrap();
        payload.as_object_mut().unwrap().remove("authentication");
        let mut mac = HmacSha256::new_from_slice(&TEST_KEY).unwrap();
        mac.update(&canonical::canonicalize(&payload).unwrap());
        artifact.authentication.signature =
            format!("hmac-sha256:{}", hex::encode(mac.finalize().into_bytes()));
        let verifier = ArtifactVerifier::new(
            "broker.workspace-write/v1",
            "filesystem.write",
            key_path,
            snapshot_path,
        );
        (directory, verifier, artifact)
    }

    #[test]
    fn verifies_bound_artifact_without_policy_resolution() {
        let now = Utc::now();
        let (_directory, verifier, artifact) = fixture(now);
        let verified = verifier.verify(artifact, now).unwrap();
        assert_eq!(
            verified.artifact().authorization.decision,
            Authorization::Allow
        );
    }

    #[test]
    fn rejects_tamper_and_expiry() {
        let now = Utc::now();
        let (_directory, verifier, mut artifact) = fixture(now);
        artifact.principal = "principal.attacker".into();
        assert!(matches!(
            verifier.verify(artifact, now),
            Err(ArtifactError::Signature)
        ));
        let (_directory, verifier, mut artifact) = fixture(now);
        artifact.expires_at =
            (now - Duration::minutes(1)).to_rfc3339_opts(SecondsFormat::Secs, true);
        let mut payload = serde_json::to_value(&artifact).unwrap();
        payload.as_object_mut().unwrap().remove("authentication");
        let mut mac = HmacSha256::new_from_slice(&TEST_KEY).unwrap();
        mac.update(&canonical::canonicalize(&payload).unwrap());
        artifact.authentication.signature =
            format!("hmac-sha256:{}", hex::encode(mac.finalize().into_bytes()));
        assert!(matches!(
            verifier.verify(artifact, now),
            Err(ArtifactError::Expired)
        ));
    }

    #[test]
    fn rejects_an_authentic_artifact_whose_layer_2_decision_is_deny() {
        let now = Utc::now();
        let (_directory, verifier, mut artifact) = fixture(now);
        artifact.authorization.decision = Authorization::Deny;
        let mut payload = serde_json::to_value(&artifact).unwrap();
        payload.as_object_mut().unwrap().remove("authentication");
        let mut mac = HmacSha256::new_from_slice(&TEST_KEY).unwrap();
        mac.update(&canonical::canonicalize(&payload).unwrap());
        artifact.authentication.signature =
            format!("hmac-sha256:{}", hex::encode(mac.finalize().into_bytes()));

        assert!(matches!(
            verifier.verify(artifact, now),
            Err(ArtifactError::AuthorizationDenied)
        ));
    }

    #[test]
    fn verifies_the_full_broker_owned_binding() {
        let now = Utc::now();
        let (_directory, verifier, artifact) = fixture(now);
        let binding = ArtifactBinding {
            principal: "principal.test".into(),
            session_id: "session.test".into(),
            targets: vec!["path:src/main.rs".into()],
            trusted_context_digest: canonical::digest(&json!([])).unwrap(),
            execution_binding: json!({}),
            boundary_generation: None,
        };
        verifier
            .verify_bound(artifact.clone(), now, &binding)
            .unwrap();
        let incorrect = ArtifactBinding {
            principal: "principal.other".into(),
            ..binding
        };
        assert!(matches!(
            verifier.verify_bound(artifact, now, &incorrect),
            Err(ArtifactError::ExecutionBinding)
        ));
    }
}
