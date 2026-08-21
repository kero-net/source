use chrono::{Duration, SecondsFormat, Utc};
use kero_core::Capability;
use kero_core::enforcement::{
    AdmissionError, Attempt, AuditEvent, AuditLog, BrokerState, CapabilityObservation, Checkpoint,
    Lifecycle, NonceStore, StateError, admit, recover_state, verify_checkpoint, write_checkpoint,
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

fn capability(now: chrono::DateTime<Utc>) -> CapabilityObservation {
    CapabilityObservation {
        schema: "kero/capability-observation/v1".into(),
        capability: Capability::Can,
        mechanism: "fixture/v1".into(),
        observed_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        expires_at: (now + Duration::minutes(1)).to_rfc3339_opts(SecondsFormat::Secs, true),
    }
}

fn checkpoint(state: &BrokerState, key: &[u8], generation: u64) {
    let audit = AuditLog::new(state.audit_path());
    write_checkpoint(
        &state.checkpoint_path(),
        Checkpoint {
            schema: "kero/checkpoint/v1".into(),
            generation,
            audit_head: audit.verify().unwrap(),
            snapshot_digest: "sha256:snapshot".into(),
            boundary: "broker.test/v1".into(),
            signature: String::new(),
        },
        key,
    )
    .unwrap();
}

#[test]
fn shared_admission_blocks_every_failed_evidence_type_before_side_effect() {
    let now = Utc::now();
    let key = [6_u8; 32];
    let directory = tempdir().unwrap();
    let state = BrokerState::open(directory.path().join("state")).unwrap();
    checkpoint(&state, &key, 0);
    let side_effect = std::cell::Cell::new(false);
    let admitted = admit(
        &state,
        attempt(0),
        &capability(now),
        now,
        &state.checkpoint_path(),
        &key,
    );
    if admitted.is_ok() {
        side_effect.set(true);
    }
    assert!(side_effect.get());

    checkpoint(&state, &key, 0);
    assert!(matches!(
        admit(
            &state,
            attempt(0),
            &capability(now),
            now,
            &state.checkpoint_path(),
            &key,
        ),
        Err(AdmissionError::Nonce(_))
    ));

    let cap_failure = CapabilityObservation {
        capability: Capability::Cannot,
        ..capability(now)
    };
    assert!(matches!(
        admit(
            &state,
            Attempt {
                nonce: "b".repeat(64),
                ..attempt(0)
            },
            &cap_failure,
            now,
            &state.checkpoint_path(),
            &key
        ),
        Err(AdmissionError::Capability)
    ));
    assert!(matches!(
        admit(
            &state,
            Attempt {
                nonce: "c".repeat(64),
                ..attempt(0)
            },
            &capability(now),
            now,
            &state.checkpoint_path(),
            &[1; 32]
        ),
        Err(AdmissionError::Checkpoint(_))
    ));
    std::fs::write(state.audit_path(), b"broken\n").unwrap();
    assert!(matches!(
        admit(
            &state,
            Attempt {
                nonce: "d".repeat(64),
                ..attempt(0)
            },
            &capability(now),
            now,
            &state.checkpoint_path(),
            &key
        ),
        Err(AdmissionError::Audit(_))
    ));
}

#[test]
fn checked_in_enforcement_fixtures_are_safe_and_parseable() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let capability: CapabilityObservation = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/enforcement/capability-current.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(capability.capability, Capability::Can);
    let audit: AuditEvent = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/enforcement/audit-genesis.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(audit.attempt.boundary, "broker.fixture/v1");
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
            schema: "kero/checkpoint/v1".into(),
            generation: 1,
            audit_head: "sha256:audit".into(),
            snapshot_digest: "sha256:snapshot".into(),
            boundary: "broker.test/v1".into(),
            signature: String::new(),
        },
        &key,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    }
    assert_eq!(verify_checkpoint(&path, &key).unwrap().generation, 1);
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("broker.test/v1", "broker.other/v1");
    fs::write(&path, text).unwrap();
    assert!(verify_checkpoint(&path, &key).is_err());
}

#[test]
fn recovery_requires_matching_signed_checkpoint_and_never_rewinds_generation() {
    let directory = tempdir().unwrap();
    let state = BrokerState::open(directory.path().join("state")).unwrap();
    let audit = AuditLog::new(state.audit_path());
    audit.append(attempt(0), Lifecycle::Prepared).unwrap();
    let generation = state.disable().unwrap();
    let path = state.checkpoint_path();
    let key = [9_u8; 32];
    write_checkpoint(
        &path,
        Checkpoint {
            schema: "kero/checkpoint/v1".into(),
            generation,
            audit_head: audit.verify().unwrap(),
            snapshot_digest: "sha256:snapshot".into(),
            boundary: "broker.test/v1".into(),
            signature: String::new(),
        },
        &key,
    )
    .unwrap();
    let checkpoint = verify_checkpoint(&path, &key).unwrap();
    assert!(
        recover_state(
            &state,
            &checkpoint,
            &audit,
            "sha256:other",
            "broker.test/v1",
        )
        .is_err()
    );
    recover_state(
        &state,
        &checkpoint,
        &audit,
        "sha256:snapshot",
        "broker.test/v1",
    )
    .unwrap();
    state.require_active(generation).unwrap();
    assert!(state.require_active(0).is_err());
}
