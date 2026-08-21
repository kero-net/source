use chrono::Utc;
use kero_core::canonical;
use kero_core::policy::{authorize, load_policy, load_request};
use kero_core::{ArtifactIssueOptions, issue_artifact};
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn supported_release_commands_are_listed_and_invalid_write_is_structured() {
    let binary = env!("CARGO_BIN_EXE_kero");
    let help = Command::new(binary).arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("workspace-write"));
    assert!(text.contains("bubblewrap"));
    assert!(text.contains("git-push"));
    assert!(text.contains("https-service"));
    for command in [
        "init",
        "global",
        "project",
        "knowledge",
        "inspect",
        "policy",
        "verify-artifact",
        "workspace-write",
        "bubblewrap",
        "git-push",
        "https-service",
    ] {
        let output = Command::new(binary)
            .args([command, "--help"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{command} help must be available");
    }
    assert!(text.contains("verify-artifact"));

    let invalid = Command::new(binary)
        .args(["workspace-write", "--root", "/missing"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(
        String::from_utf8(invalid.stderr)
            .unwrap()
            .contains("required arguments")
    );
}

#[test]
fn workspace_write_cli_performs_one_exact_artifact_bound_write() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/policy");
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
    fs::write(&key, [8_u8; 32]).unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let content = b"cli-bound-content\n";
    let artifact = issue_artifact(
        &decision,
        &request,
        &snapshot,
        &key,
        ArtifactIssueOptions {
            ttl_seconds: 60,
            now: Utc::now(),
            audience: "broker.workspace-write/v1".into(),
            execution_binding: json!({
                "path": "path:src/main.rs",
                "content_digest": canonical::sha256(content),
            }),
            boundary_generation: Some(0),
        },
    )
    .unwrap();
    let artifact_path = directory.path().join("artifact.json");
    fs::write(&artifact_path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let content_path = directory.path().join("content");
    fs::write(&content_path, content).unwrap();
    let workspace = directory.path().join("workspace");
    fs::create_dir_all(workspace.join("src")).unwrap();
    let state = directory.path().join("state");
    let output = Command::new(env!("CARGO_BIN_EXE_kero"))
        .args([
            "workspace-write",
            "--root",
            workspace.to_str().unwrap(),
            "--state",
            state.to_str().unwrap(),
            "--artifact",
            artifact_path.to_str().unwrap(),
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
            "--content",
            content_path.to_str().unwrap(),
            "--target",
            "path:src/main.rs",
            "--principal",
            &request.principal,
            "--session",
            request.session.get("id").unwrap().as_str().unwrap(),
            "--context-digest",
            &canonical::digest(&request.context).unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(fs::read(workspace.join("src/main.rs")).unwrap(), content);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("workspace.write.completed")
    );
}
