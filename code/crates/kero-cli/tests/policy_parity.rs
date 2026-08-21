use kero_core::canonical;
use kero_core::policy::{authorize, load_policy, load_request, load_snapshot};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/policy")
}

#[test]
fn python_reference_snapshot_fixture_is_byte_compatible() {
    let fixtures = fixtures();
    let policy = load_policy(
        &fixtures.join("environment.toml"),
        &[fixtures.join("records.toml")],
    )
    .unwrap();
    let request = load_request(&fixtures.join("request.toml")).unwrap();
    let store = tempdir().unwrap();
    let result = authorize(&policy, &request, store.path()).unwrap();

    assert_eq!(result.decision, "ALLOW");
    assert_eq!(
        result.snapshot.digest,
        "sha256:30fc55f107ca833739efaac21444a072017710a3e5e22f138bfca5bcb6c4abed"
    );
}

#[test]
fn immutable_snapshot_replay_is_byte_equivalent_without_toml() {
    let fixtures = fixtures();
    let policy = load_policy(
        &fixtures.join("environment.toml"),
        &[fixtures.join("records.toml")],
    )
    .unwrap();
    let request = load_request(&fixtures.join("request.toml")).unwrap();
    let store = tempdir().unwrap();
    let original = authorize(&policy, &request, store.path()).unwrap();
    let snapshot = original.snapshot.retrieval.strip_prefix("file:").unwrap();
    let replay_policy = load_snapshot(snapshot.as_ref(), Some(&original.snapshot.digest)).unwrap();
    let replayed = authorize(&replay_policy, &request, store.path()).unwrap();

    assert_eq!(
        canonical::canonicalize(&original).unwrap(),
        canonical::canonicalize(&replayed).unwrap()
    );
}

#[derive(Deserialize)]
struct ExpectedDecision {
    decision: String,
    reason: String,
}

#[test]
fn immutable_decision_table_covers_allow_deny_and_indeterminate_precedence() {
    let fixtures = fixtures();
    let policy = load_policy(
        &fixtures.join("environment.toml"),
        &[fixtures.join("records.toml")],
    )
    .unwrap();
    let decisions = fixtures.join("decisions");
    for name in [
        "allow",
        "explicit-deny",
        "indeterminate-deny",
        "target-type-mismatch",
    ] {
        let request = load_request(&decisions.join(format!("{name}.request.toml"))).unwrap();
        let expected: ExpectedDecision = serde_json::from_slice(
            &fs::read(decisions.join(format!("{name}.result.json"))).unwrap(),
        )
        .unwrap();
        let store = tempdir().unwrap();
        let actual = authorize(&policy, &request, store.path()).unwrap();
        assert_eq!(actual.decision, expected.decision, "fixture {name}");
        assert_eq!(actual.reason, expected.reason, "fixture {name}");
    }
}
