use scope_cli::enforcement::{
    Attempt, AuditLog, BrokerState, Checkpoint, Lifecycle, NonceStore, StateError,
    verify_checkpoint, write_checkpoint,
};
use std::fs;
use tempfile::tempdir;

fn attempt(generation: u64) -> Attempt {
    Attempt {
        artifact_id: "artifact.test".into(),
        nonce: "a".repeat(64),
        boundary: "broker.test/v1".into(),
        operation: "filesystem.write".into(),
        target_digest: "sha256:target".into(),
        snapshot_digest: "sha256:snapshot".into(),
        generation,
    }
}

#[test]
fn durable_nonce_admission_rejects_replay_and_disabled_state() {
    let directory = tempdir().unwrap();
    let state = BrokerState::open(directory.path()).unwrap();
    let nonces = NonceStore::new(state.clone());
    nonces.admit(&"a".repeat(64), 0).unwrap();
    assert!(
        nonces
            .admit(&"a".repeat(64), 0)
            .unwrap_err()
            .to_string()
            .contains("nonce.replay")
    );
    let generation = state.disable().unwrap();
    assert!(matches!(
        state.require_active(generation),
        Err(StateError::Disabled)
    ));
    assert!(nonces.admit(&"b".repeat(64), generation).is_err());
}

#[test]
fn audit_chain_rejects_corruption_before_a_new_event() {
    let directory = tempdir().unwrap();
    let state = BrokerState::open(directory.path()).unwrap();
    let audit = AuditLog::new(state.audit_path());
    audit.append(attempt(0), Lifecycle::Prepared).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(state.audit_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    audit.append(attempt(0), Lifecycle::Completed).unwrap();
    assert!(audit.verify().unwrap().starts_with("sha256:"));
    fs::write(state.audit_path(), b"{\"broken\":true}\n").unwrap();
    assert!(audit.append(attempt(0), Lifecycle::Blocked).is_err());
}

#[test]
fn signed_checkpoint_detects_tamper() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("checkpoint.json");
    let key = [7_u8; 32];
    write_checkpoint(
        &path,
        Checkpoint {
            schema: "scope/checkpoint/v1".into(),
            generation: 1,
            audit_head: "sha256:audit".into(),
            snapshot_digest: "sha256:snapshot".into(),
            boundary: "broker.test/v1".into(),
            signature: String::new(),
        },
        &key,
    )
    .unwrap();
    assert_eq!(verify_checkpoint(&path, &key).unwrap().generation, 1);
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("broker.test/v1", "broker.other/v1");
    fs::write(&path, text).unwrap();
    assert!(verify_checkpoint(&path, &key).is_err());
}
