use chrono::Utc;
use scope_cli::canonical;
use scope_cli::policy::{authorize, load_policy, load_request};
use scope_cli::{ArtifactIssueOptions, ArtifactVerifier, WorkspaceWriteBroker, issue_artifact};
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tempfile::tempdir;

const KEY: [u8; 32] = [3; 32];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/policy")
}

#[test]
fn exact_descriptor_relative_write_is_bound_and_single_use() {
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
    fs::write(&key, KEY).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let body = b"approved contents\n";
    let now = Utc::now();
    let artifact = issue_artifact(
        &decision,
        &request,
        &snapshot,
        &key,
        ArtifactIssueOptions {
            ttl_seconds: 300,
            now,
            audience: "broker.workspace-write/v1".into(),
            execution_binding: json!({
                "path": "path:src/main.rs",
                "content_digest": canonical::sha256(body),
            }),
            boundary_generation: Some(0),
        },
    )
    .unwrap();
    let artifact_path = directory.path().join("artifact.json");
    fs::write(&artifact_path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir_all(workspace.join("src")).unwrap();
    let broker = WorkspaceWriteBroker::new(&workspace, directory.path().join("state")).unwrap();
    let verifier = ArtifactVerifier::new(
        "broker.workspace-write/v1",
        "filesystem.write",
        &key,
        snapshot,
    );
    assert_eq!(
        broker
            .write(&verifier, &artifact_path, body)
            .unwrap()
            .reason,
        "workspace.write.completed"
    );
    assert_eq!(fs::read(workspace.join("src/main.rs")).unwrap(), body);
    assert!(
        broker
            .write(&verifier, &artifact_path, body)
            .unwrap_err()
            .to_string()
            .contains("workspace.replay")
    );
}

#[test]
fn symlinked_parent_is_not_followed() {
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
    fs::write(&key, KEY).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let body = b"must not escape\n";
    let artifact = issue_artifact(
        &decision,
        &request,
        &snapshot,
        &key,
        ArtifactIssueOptions {
            ttl_seconds: 300,
            now: Utc::now(),
            audience: "broker.workspace-write/v1".into(),
            execution_binding: json!({
                "path": "path:src/main.rs",
                "content_digest": canonical::sha256(body),
            }),
            boundary_generation: Some(0),
        },
    )
    .unwrap();
    let artifact_path = directory.path().join("artifact.json");
    fs::write(&artifact_path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let workspace = directory.path().join("workspace");
    let outside = directory.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, workspace.join("src")).unwrap();
    let broker = WorkspaceWriteBroker::new(&workspace, directory.path().join("state")).unwrap();
    let verifier = ArtifactVerifier::new(
        "broker.workspace-write/v1",
        "filesystem.write",
        &key,
        snapshot,
    );
    assert!(
        broker
            .write(&verifier, &artifact_path, body)
            .unwrap_err()
            .to_string()
            .contains("workspace.target-invalid")
    );
    assert!(!outside.join("main.rs").exists());
}
