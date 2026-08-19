use scope_cli::policy::{authorize, load_policy, load_request};
use scope_cli::{ArtifactIssueOptions, ArtifactVerifier, issue_artifact};
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::tempdir;
use time::OffsetDateTime;

const TEST_KEY: [u8; 32] = [0; 32];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/policy")
}

#[test]
fn layer_2_allow_issues_an_artifact_the_layer_3_verifier_accepts() {
    let fixtures = fixtures();
    let policy = load_policy(
        &fixtures.join("environment.toml"),
        &[fixtures.join("records.toml")],
    )
    .unwrap();
    let request = load_request(&fixtures.join("request.toml")).unwrap();
    let directory = tempdir().unwrap();
    let decision = authorize(&policy, &request, &directory.path().join("snapshots")).unwrap();
    let snapshot = PathBuf::from(decision.snapshot.retrieval.strip_prefix("file:").unwrap());
    let key = directory.path().join("artifact.key");
    fs::write(&key, TEST_KEY).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let now = OffsetDateTime::now_utc();
    let artifact = issue_artifact(
        &decision,
        &request,
        &snapshot,
        &key,
        ArtifactIssueOptions {
            ttl_seconds: 300,
            now,
            audience: "broker.workspace-write/v1".into(),
            execution_binding: json!({}),
            boundary_generation: None,
        },
    )
    .unwrap();

    let verified = ArtifactVerifier::new(
        "broker.workspace-write/v1",
        "filesystem.write",
        key,
        snapshot,
    )
    .verify(artifact, now)
    .unwrap();
    assert_eq!(verified.artifact().principal, "principal.example");
    assert_eq!(verified.artifact().session_id, "session.example");
}

#[test]
fn layer_2_deny_cannot_be_materialized_as_executable_authority() {
    let fixtures = fixtures();
    let policy = load_policy(
        &fixtures.join("environment.toml"),
        &[fixtures.join("records.toml")],
    )
    .unwrap();
    let request = load_request(&fixtures.join("request.toml")).unwrap();
    let directory = tempdir().unwrap();
    let mut decision = authorize(&policy, &request, &directory.path().join("snapshots")).unwrap();
    decision.decision = "DENY".into();
    let snapshot = PathBuf::from(decision.snapshot.retrieval.strip_prefix("file:").unwrap());
    let key = directory.path().join("artifact.key");
    fs::write(&key, TEST_KEY).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();

    assert!(
        issue_artifact(
            &decision,
            &request,
            &snapshot,
            &key,
            ArtifactIssueOptions {
                ttl_seconds: 300,
                now: OffsetDateTime::now_utc(),
                audience: "broker.workspace-write/v1".into(),
                execution_binding: json!({}),
                boundary_generation: None,
            }
        )
        .is_err()
    );
}
