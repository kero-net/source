use super::model::{PolicySet, RESOLVER_VERSION, SNAPSHOT_SCHEMA};
use crate::canonical;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("snapshot.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("snapshot.serialization: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Canonical(#[from] canonical::CanonicalError),
    #[error("snapshot.schema-unsupported")]
    Schema,
    #[error("snapshot.resolver-unsupported")]
    Resolver,
    #[error("snapshot.digest-mismatch")]
    Digest,
    #[error("snapshot.canonical-mismatch")]
    CanonicalMismatch,
    #[error("snapshot.immutable-collision")]
    Collision,
    #[error("snapshot.policy-invalid: {0}")]
    PolicyInvalid(#[from] super::load::PolicyLoadError),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Snapshot {
    schema: String,
    resolver_version: String,
    scope_implementations: BTreeMap<String, String>,
    environment: super::model::PolicyEnvironment,
    records: Records,
    diagnostics: Vec<super::model::Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Records {
    principals: Vec<super::model::Principal>,
    roles: Vec<super::model::Role>,
    statements: Vec<super::model::Statement>,
    assignments: Vec<super::model::Assignment>,
    delegations: Vec<super::model::Delegation>,
}

fn payload(policy: &PolicySet) -> Snapshot {
    Snapshot {
        schema: SNAPSHOT_SCHEMA.into(),
        resolver_version: RESOLVER_VERSION.into(),
        scope_implementations: [
            "branch",
            "command",
            "environment",
            "knowledge",
            "path",
            "repository",
            "service",
        ]
        .into_iter()
        .map(|kind| (kind.into(), "builtin/v1".into()))
        .collect(),
        environment: policy.environment.clone(),
        records: Records {
            principals: policy.principals.values().cloned().collect(),
            roles: policy.roles.values().cloned().collect(),
            statements: policy.statements.values().cloned().collect(),
            assignments: policy.assignments.values().cloned().collect(),
            delegations: policy.delegations.values().cloned().collect(),
        },
        diagnostics: policy.diagnostics.clone(),
    }
}

pub fn snapshot_bytes(policy: &PolicySet) -> Result<Vec<u8>, SnapshotError> {
    let mut bytes = canonical::canonicalize(&payload(policy))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn snapshot_digest(policy: &PolicySet) -> Result<String, SnapshotError> {
    Ok(canonical::sha256(&snapshot_bytes(policy)?))
}

pub fn write_snapshot(
    policy: &PolicySet,
    store: &Path,
) -> Result<(String, String, PathBuf), SnapshotError> {
    super::load::validate_policy(policy)?;
    let content = snapshot_bytes(policy)?;
    let digest = canonical::sha256(&content);
    let hex = digest.strip_prefix("sha256:").expect("digest prefix");
    let id = format!("snapshot.{}", &hex[..16]);
    fs::create_dir_all(store)?;
    let destination = store.join(format!("{hex}.json"));
    if destination.exists() {
        return if fs::read(&destination)? == content {
            Ok((id, digest, destination))
        } else {
            Err(SnapshotError::Collision)
        };
    }
    let temporary = store.join(format!(".snapshot-{}-{hex}", std::process::id()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&content)?;
    file.sync_all()?;
    match fs::hard_link(&temporary, &destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(&destination)? != content {
                fs::remove_file(&temporary)?;
                return Err(SnapshotError::Collision);
            }
        }
        Err(error) => {
            fs::remove_file(&temporary)?;
            return Err(error.into());
        }
    }
    fs::remove_file(&temporary)?;
    Ok((id, digest, destination))
}

pub fn load_snapshot(
    path: &Path,
    expected_digest: Option<&str>,
) -> Result<PolicySet, SnapshotError> {
    let bytes = fs::read(path)?;
    let actual = canonical::sha256(&bytes);
    if expected_digest.is_some_and(|expected| expected != actual) {
        return Err(SnapshotError::Digest);
    }
    let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
    if snapshot.schema != SNAPSHOT_SCHEMA {
        return Err(SnapshotError::Schema);
    }
    if snapshot.resolver_version != RESOLVER_VERSION {
        return Err(SnapshotError::Resolver);
    }
    let mut policy = PolicySet {
        environment: snapshot.environment,
        principals: snapshot
            .records
            .principals
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect(),
        roles: snapshot
            .records
            .roles
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect(),
        statements: snapshot
            .records
            .statements
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect(),
        assignments: snapshot
            .records
            .assignments
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect(),
        delegations: snapshot
            .records
            .delegations
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect(),
        diagnostics: snapshot.diagnostics,
    };
    policy.diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then(left.record_id.cmp(&right.record_id))
    });
    if snapshot_bytes(&policy)? != bytes {
        return Err(SnapshotError::CanonicalMismatch);
    }
    super::load::validate_policy(&policy)?;
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::load::{load_policy, load_request};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn snapshot_is_immutable_and_replayable() {
        let directory = tempdir().unwrap();
        let environment = directory.path().join("environment.toml");
        let records = directory.path().join("records.toml");
        fs::write(&environment, crate::policy::resolve::tests::ENVIRONMENT).unwrap();
        fs::write(&records, crate::policy::resolve::tests::RECORDS).unwrap();
        let policy = load_policy(&environment, &[&records]).unwrap();
        let (id, digest, path) =
            write_snapshot(&policy, &directory.path().join("snapshots")).unwrap();
        assert!(id.starts_with("snapshot."));
        assert_eq!(load_snapshot(&path, Some(&digest)).unwrap(), policy);
        assert!(load_snapshot(&path, Some("sha256:wrong")).is_err());
        let _ = load_request;
    }

    #[test]
    fn invalid_policy_cannot_be_written_as_a_snapshot() {
        let directory = tempdir().unwrap();
        let environment = directory.path().join("environment.toml");
        let records = directory.path().join("records.toml");
        fs::write(&environment, crate::policy::resolve::tests::ENVIRONMENT).unwrap();
        fs::write(&records, crate::policy::resolve::tests::RECORDS).unwrap();
        let mut policy = load_policy(&environment, &[&records]).unwrap();
        policy
            .roles
            .values_mut()
            .next()
            .unwrap()
            .inherits
            .push("role.missing".into());
        let error = write_snapshot(&policy, &directory.path().join("snapshots")).unwrap_err();
        assert!(error.to_string().contains("role.inherited-missing"));
    }

    #[test]
    fn replay_rejects_noncanonical_snapshot_bytes() {
        let directory = tempdir().unwrap();
        let environment = directory.path().join("environment.toml");
        let records = directory.path().join("records.toml");
        fs::write(&environment, crate::policy::resolve::tests::ENVIRONMENT).unwrap();
        fs::write(&records, crate::policy::resolve::tests::RECORDS).unwrap();
        let policy = load_policy(&environment, &[&records]).unwrap();
        let (_, _, path) = write_snapshot(&policy, &directory.path().join("snapshots")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(matches!(
            load_snapshot(&path, None),
            Err(SnapshotError::CanonicalMismatch)
        ));
    }

    #[test]
    fn immutable_store_rejects_existing_content_at_digest_path() {
        let directory = tempdir().unwrap();
        let environment = directory.path().join("environment.toml");
        let records = directory.path().join("records.toml");
        fs::write(&environment, crate::policy::resolve::tests::ENVIRONMENT).unwrap();
        fs::write(&records, crate::policy::resolve::tests::RECORDS).unwrap();
        let policy = load_policy(&environment, &[&records]).unwrap();
        let content = snapshot_bytes(&policy).unwrap();
        let digest = canonical::sha256(&content);
        let store = directory.path().join("snapshots");
        fs::create_dir_all(&store).unwrap();
        fs::write(
            store.join(format!("{}.json", digest.strip_prefix("sha256:").unwrap())),
            b"different immutable content",
        )
        .unwrap();
        assert!(matches!(
            write_snapshot(&policy, &store),
            Err(SnapshotError::Collision)
        ));
    }
}
