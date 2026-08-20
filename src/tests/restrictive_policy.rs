use scope_cli::policy::{authorize, load_policy, load_request};
use std::fs;
use tempfile::tempdir;

#[test]
fn restrictive_deny_only_closure_can_narrow_but_cannot_grant() {
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let origin = directory.path().join("origin.toml");
    let restrictive = directory.path().join("restrictive.toml");
    let request = directory.path().join("request.toml");
    fs::write(
        &environment,
        r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.restrictive"
[[sources]]
id = "source.origin"
kind = "global-policy"
locator = "origin.toml"
trust_mode = "origin"
record_actions = ["principal.define", "role.define", "role.assign", "statement.allow"]
operations = ["*"]
scope_universe = ["path"]
[[sources]]
id = "source.restrictive"
kind = "project-policy"
locator = "restrictive.toml"
trust_mode = "restrictive"
record_actions = ["role.define", "role.assign", "statement.deny"]
operations = ["filesystem.write"]
scopes = ["path:src/protected/**"]
scope_universe = ["path"]
"#,
    )
    .unwrap();
    fs::write(
        &origin,
        r#"schema = "terminal-policy/records/v1"
source = "source.origin"
[[principals]]
id = "principal.user"
[[roles]]
id = "role.contributor"
[[statements]]
id = "statement.allow-write"
role = "role.contributor"
effect = "allow"
operations = ["filesystem.write"]
scopes = ["path:src/**"]
[[assignments]]
id = "assignment.user.contributor"
principal = "principal.user"
roles = ["role.contributor"]
operations = ["filesystem.write"]
scopes = ["path:src/**"]
"#,
    )
    .unwrap();
    fs::write(
        &restrictive,
        r#"schema = "terminal-policy/records/v1"
source = "source.restrictive"
[[roles]]
id = "role.protected-deny"
[[statements]]
id = "statement.deny-protected"
role = "role.protected-deny"
effect = "deny"
operations = ["filesystem.write"]
scopes = ["path:src/protected/**"]
[[assignments]]
id = "assignment.user.protected-deny"
principal = "principal.user"
roles = ["role.protected-deny"]
operations = ["filesystem.write"]
scopes = ["path:src/protected/**"]
"#,
    )
    .unwrap();
    fs::write(
        &request,
        r#"schema = "terminal-policy/request/v1"
request_id = "request.restrictive"
principal = "principal.user"
operation = "filesystem.write"
targets = ["path:src/protected/main.rs"]
[session]
id = "session.test"
"#,
    )
    .unwrap();

    let policy = load_policy(&environment, &[&origin, &restrictive]).unwrap();
    let result = authorize(
        &policy,
        &load_request(&request).unwrap(),
        &directory.path().join("snapshots"),
    )
    .unwrap();
    assert_eq!(
        (result.decision.as_str(), result.reason.as_str()),
        ("DENY", "deny.explicit")
    );

    let invalid = fs::read_to_string(&restrictive)
        .unwrap()
        .replace("effect = \"deny\"", "effect = \"allow\"");
    fs::write(&restrictive, invalid).unwrap();
    let error = load_policy(&environment, &[&origin, &restrictive])
        .unwrap_err()
        .to_string();
    assert!(error.contains("source.unauthorized"));
}
