use kero_core::policy::{load_policy, load_request};
use std::fs;
use tempfile::tempdir;

const ENVIRONMENT: &str = r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.validation"
context_providers = ["time", "project"]

[[sources]]
id = "source.origin"
kind = "project-policy"
locator = "records.toml"
trust_mode = "origin"
record_actions = ["principal.define", "role.define", "role.assign", "statement.allow", "statement.deny", "delegation.issue"]
operations = ["*"]
scope_universe = ["path"]
"#;

const RECORDS: &str = r#"schema = "terminal-policy/records/v1"
source = "source.origin"

[[principals]]
id = "principal.user"

[[roles]]
id = "role.parent"

[[roles]]
id = "role.child"
inherits = ["role.parent"]

[[statements]]
id = "statement.write"
role = "role.child"
effect = "allow"
operations = ["filesystem.write"]
scopes = ["path:src/**"]

[[assignments]]
id = "assignment.user"
principal = "principal.user"
roles = ["role.child"]
operations = ["filesystem.write"]
scopes = ["path:src/**"]
"#;

fn load(records: &str) -> Result<(), String> {
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let records_path = directory.path().join("records.toml");
    fs::write(&environment, ENVIRONMENT).unwrap();
    fs::write(&records_path, records).unwrap();
    load_policy(&environment, &[records_path])
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn accepts_a_structurally_complete_authorized_policy() {
    load(RECORDS).unwrap();
}

#[test]
fn rejects_malformed_ids_and_unsupported_effects() {
    let malformed = RECORDS.replace("role.child", "Role Child");
    assert!(load(&malformed).unwrap_err().contains("id.malformed"));

    let effect = RECORDS.replace("effect = \"allow\"", "effect = \"permit\"");
    assert!(
        load(&effect)
            .unwrap_err()
            .contains("statement.effect-unsupported")
    );
}

#[test]
fn rejects_cross_type_duplicate_ids_and_unknown_record_actions() {
    let duplicate = RECORDS.replace("id = \"statement.write\"", "id = \"role.child\"");
    assert!(
        load(&duplicate)
            .unwrap_err()
            .contains("policy.id-duplicate: role.child")
    );

    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let records = directory.path().join("records.toml");
    fs::write(
        &environment,
        ENVIRONMENT.replace(
            "\"delegation.issue\"]",
            "\"delegation.issue\", \"role.superuser\"]",
        ),
    )
    .unwrap();
    fs::write(&records, RECORDS).unwrap();
    let error = load_policy(&environment, &[records])
        .unwrap_err()
        .to_string();
    assert!(error.contains("source.record-action-unsupported"));
}

#[test]
fn rejects_unknown_schema_fields_instead_of_ignoring_restrictions() {
    let misspelled = RECORDS.replace(
        "scopes = [\"path:src/**\"]\n\n[[assignments]]",
        "kero = \"path:src/**\"\n\n[[assignments]]",
    );
    let error = load(&misspelled).unwrap_err();
    assert!(error.contains("unknown field `kero`"));
}

#[test]
fn rejects_malformed_operation_selectors_and_unknown_scope_universe_types() {
    let operation = RECORDS.replace("filesystem.write", "filesystem*write");
    assert!(
        load(&operation)
            .unwrap_err()
            .contains("operation.selector-invalid")
    );

    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let records = directory.path().join("records.toml");
    fs::write(
        &environment,
        ENVIRONMENT.replace(
            "scope_universe = [\"path\"]",
            "scope_universe = [\"filesystem\"]",
        ),
    )
    .unwrap();
    fs::write(&records, RECORDS).unwrap();
    let error = load_policy(&environment, &[records])
        .unwrap_err()
        .to_string();
    assert!(error.contains("scope.universe-type-unknown"));
}

#[test]
fn rejects_unsupported_schema_versions() {
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let records = directory.path().join("records.toml");
    fs::write(
        &environment,
        ENVIRONMENT.replace(
            "terminal-policy/environment/v1",
            "terminal-policy/environment/v2",
        ),
    )
    .unwrap();
    fs::write(&records, RECORDS).unwrap();
    assert!(
        load_policy(&environment, &[&records])
            .unwrap_err()
            .to_string()
            .contains("policy.schema-unsupported")
    );

    fs::write(&environment, ENVIRONMENT).unwrap();
    fs::write(
        &records,
        RECORDS.replace("terminal-policy/records/v1", "terminal-policy/records/v2"),
    )
    .unwrap();
    assert!(
        load_policy(&environment, &[&records])
            .unwrap_err()
            .to_string()
            .contains("policy.schema-unsupported")
    );
}

#[test]
fn rejects_missing_references_and_role_cycles() {
    let missing = RECORDS.replace("roles = [\"role.child\"]", "roles = [\"role.missing\"]");
    assert!(
        load(&missing)
            .unwrap_err()
            .contains("assignment.role-missing")
    );

    let cycle = RECORDS.replace(
        "id = \"role.parent\"",
        "id = \"role.parent\"\ninherits = [\"role.child\"]",
    );
    assert!(load(&cycle).unwrap_err().contains("role.inheritance-cycle"));
}

#[test]
fn rejects_unknown_condition_providers_and_unauthorized_records() {
    let condition = RECORDS.replace(
        "scopes = [\"path:src/**\"]\n\n[[assignments]]",
        "scopes = [\"path:src/**\"]\nconditions = [{ provider = \"client\", key = \"id\", operator = \"eq\", value = \"kero\" }]\n\n[[assignments]]",
    );
    assert!(
        load(&condition)
            .unwrap_err()
            .contains("condition.provider-unknown")
    );

    let unauthorized_environment = ENVIRONMENT.replace("\"statement.allow\", ", "");
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let records = directory.path().join("records.toml");
    fs::write(&environment, unauthorized_environment).unwrap();
    fs::write(&records, RECORDS).unwrap();
    let error = load_policy(&environment, &[records])
        .unwrap_err()
        .to_string();
    assert!(error.contains("source.unauthorized:statement.write"));
}

#[test]
fn rejects_malformed_or_inverted_validity_windows() {
    let malformed = RECORDS.replace(
        "effect = \"allow\"",
        "effect = \"allow\"\nnot_before = \"tomorrow\"",
    );
    assert!(load(&malformed).unwrap_err().contains("validity.malformed"));

    let inverted = RECORDS.replace(
        "effect = \"allow\"",
        "effect = \"allow\"\nnot_before = \"2026-08-20T00:00:00Z\"\nnot_after = \"2026-08-19T00:00:00Z\"",
    );
    assert!(load(&inverted).unwrap_err().contains("validity.inverted"));
}

#[test]
fn rejects_incomplete_and_duplicate_request_context() {
    let directory = tempdir().unwrap();
    let request = directory.path().join("request.toml");
    fs::write(
        &request,
        r#"schema = "terminal-policy/request/v1"
request_id = "request.validation"
principal = "principal.user"
operation = "filesystem.write"
targets = ["path:src/main.rs"]

[[context]]
provider = "time"
key = "now"
value = "2026-08-19T00:00:00Z"
provenance = "clock.test"
digest = "sha256:test"

[[context]]
provider = "time"
key = "now"
value = "2026-08-19T00:00:00Z"
provenance = "clock.test"
digest = "sha256:test"
"#,
    )
    .unwrap();
    assert!(
        load_request(&request)
            .unwrap_err()
            .to_string()
            .contains("context.duplicate")
    );

    fs::write(
        &request,
        r#"schema = "terminal-policy/request/v1"
request_id = "request.validation"
principal = "principal.user"
operation = "filesystem.write"
targets = ["path:src/main.rs"]
[[context]]
provider = "time"
key = "now"
value = "2026-08-19T00:00:00Z"
provenance = ""
digest = ""
"#,
    )
    .unwrap();
    assert!(
        load_request(&request)
            .unwrap_err()
            .to_string()
            .contains("context.incomplete")
    );
}

#[test]
fn rejects_request_wildcards_and_duplicate_targets() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/policy/validation");
    assert!(
        load_request(&fixtures.join("wildcard-operation.request.toml"))
            .unwrap_err()
            .to_string()
            .contains("request.operation-invalid")
    );
    assert!(
        load_request(&fixtures.join("duplicate-target.request.toml"))
            .unwrap_err()
            .to_string()
            .contains("request.targets-duplicate")
    );
}
