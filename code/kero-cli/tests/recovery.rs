use kero_cli::enforcement::{Checkpoint, verify_checkpoint, write_checkpoint};
use std::fs;
use tempfile::tempdir;

#[test]
fn truncated_and_missing_checkpoint_evidence_fails_closed() {
    let directory = tempdir().unwrap();
    let key = [5_u8; 32];
    let missing = directory.path().join("missing.json");
    assert!(verify_checkpoint(&missing, &key).is_err());
    let path = directory.path().join("checkpoint.json");
    write_checkpoint(
        &path,
        Checkpoint {
            schema: "kero/checkpoint/v1".into(),
            generation: 0,
            audit_head: "sha256:genesis".into(),
            snapshot_digest: "sha256:snapshot".into(),
            boundary: "broker.fixture/v1".into(),
            signature: String::new(),
        },
        &key,
    )
    .unwrap();
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
    assert!(verify_checkpoint(&path, &key).is_err());
}
