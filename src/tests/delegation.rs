use scope_cli::policy::{authorize, load_policy, load_request};
use std::fs;
use tempfile::tempdir;

#[test]
fn delegated_allow_retains_complete_chain() {
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let global = directory.path().join("global.toml");
    let project = directory.path().join("project.toml");
    let request = directory.path().join("request.toml");
    fs::write(&environment, r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.test"
[[sources]]
id = "source.global"
kind = "global-policy"
locator = "global.toml"
trust_mode = "origin"
record_actions = ["principal.define", "role.define", "role.assign", "statement.allow", "statement.deny", "delegation.issue"]
operations = ["*"]
scope_universe = ["path"]
[[sources]]
id = "source.project"
kind = "project-policy"
locator = "project.toml"
trust_mode = "delegated"
operations = ["filesystem.write"]
scope_universe = ["path"]
"#).unwrap();
    fs::write(
        &global,
        r#"schema = "terminal-policy/records/v1"
source = "source.global"
[[principals]]
id = "principal.user"
[[delegations]]
id = "delegation.global.project"
issuer = "source.global"
recipient = "source.project"
record_actions = ["role.define", "role.assign", "statement.allow"]
principals = ["principal.user"]
roles = ["role.project.*"]
operations = ["filesystem.write"]
scopes = ["path:src/**"]
redelegation = false
max_depth = 0
"#,
    )
    .unwrap();
    fs::write(
        &project,
        r#"schema = "terminal-policy/records/v1"
source = "source.project"
[[roles]]
id = "role.project.contributor"
[[statements]]
id = "statement.project.write"
role = "role.project.contributor"
effect = "allow"
operations = ["filesystem.write"]
scopes = ["path:src/**"]
[[assignments]]
id = "assignment.project.user"
principal = "principal.user"
roles = ["role.project.contributor"]
operations = ["filesystem.write"]
scopes = ["path:src/**"]
"#,
    )
    .unwrap();
    fs::write(
        &request,
        r#"schema = "terminal-policy/request/v1"
request_id = "request.test"
principal = "principal.user"
operation = "filesystem.write"
targets = ["path:src/main.rs"]
[session]
id = "session.test"
"#,
    )
    .unwrap();

    let policy = load_policy(&environment, &[&global, &project]).unwrap();
    let result = authorize(
        &policy,
        &load_request(&request).unwrap(),
        &directory.path().join("snapshots"),
    )
    .unwrap();
    assert_eq!(
        (result.decision.as_str(), result.reason.as_str()),
        ("ALLOW", "allow.applicable")
    );
    let statement = result
        .statements
        .iter()
        .find(|item| item.id == "statement.project.write")
        .unwrap();
    assert_eq!(
        statement.delegation_chains,
        vec![vec!["delegation.global.project".to_string()]]
    );
}
