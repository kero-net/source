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

#[test]
fn multi_hop_delegation_requires_redelegation_and_remaining_depth() {
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let global = directory.path().join("global.toml");
    let intermediate = directory.path().join("intermediate.toml");
    let project = directory.path().join("project.toml");
    let request = directory.path().join("request.toml");
    fs::write(
        &environment,
        r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.multi-hop"
[[sources]]
id = "source.global"
kind = "global-policy"
locator = "global.toml"
trust_mode = "origin"
record_actions = ["principal.define", "delegation.issue"]
operations = ["*"]
scope_universe = ["path"]
[[sources]]
id = "source.intermediate"
kind = "shared-policy"
locator = "intermediate.toml"
trust_mode = "delegated"
operations = ["filesystem.write"]
scope_universe = ["path"]
[[sources]]
id = "source.project"
kind = "project-policy"
locator = "project.toml"
trust_mode = "delegated"
operations = ["filesystem.write"]
scope_universe = ["path"]
"#,
    )
    .unwrap();
    let global_policy = r#"schema = "terminal-policy/records/v1"
source = "source.global"
[[principals]]
id = "principal.user"
[[delegations]]
id = "delegation.global.intermediate"
issuer = "source.global"
recipient = "source.intermediate"
record_actions = ["delegation.issue", "role.define", "role.assign", "statement.allow"]
principals = ["principal.user"]
roles = ["role.project.*"]
operations = ["filesystem.write"]
scopes = ["path:src/**"]
redelegation = true
max_depth = 2
"#;
    fs::write(&global, global_policy).unwrap();
    fs::write(
        &intermediate,
        r#"schema = "terminal-policy/records/v1"
source = "source.intermediate"
[[delegations]]
id = "delegation.intermediate.project"
issuer = "source.intermediate"
recipient = "source.project"
record_actions = ["role.define", "role.assign", "statement.allow"]
principals = ["principal.user"]
roles = ["role.project.*"]
operations = ["filesystem.write"]
scopes = ["path:src/**"]
redelegation = false
max_depth = 1
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
request_id = "request.multi-hop"
principal = "principal.user"
operation = "filesystem.write"
targets = ["path:src/main.rs"]
[session]
id = "session.test"
"#,
    )
    .unwrap();

    let policy = load_policy(&environment, &[&global, &intermediate, &project]).unwrap();
    let result = authorize(
        &policy,
        &load_request(&request).unwrap(),
        &directory.path().join("snapshots"),
    )
    .unwrap();
    assert_eq!(result.decision, "ALLOW");
    let statement = result
        .statements
        .iter()
        .find(|item| item.id == "statement.project.write")
        .unwrap();
    assert!(statement.delegation_chains.contains(&vec![
        "delegation.global.intermediate".into(),
        "delegation.intermediate.project".into(),
    ]));

    fs::write(
        &global,
        global_policy.replace("redelegation = true", "redelegation = false"),
    )
    .unwrap();
    let error = load_policy(&environment, &[&global, &intermediate, &project])
        .unwrap_err()
        .to_string();
    assert!(error.contains("source.unauthorized"));

    fs::write(
        &global,
        r#"schema = "terminal-policy/records/v1"
source = "source.global"
[[principals]]
id = "principal.user"
[[delegations]]
id = "delegation.global.operation-only"
issuer = "source.global"
recipient = "source.intermediate"
record_actions = ["delegation.issue", "role.define", "role.assign", "statement.allow"]
principals = ["principal.user"]
roles = ["role.project.*"]
operations = ["filesystem.write"]
scopes = ["path:tests/**"]
redelegation = true
max_depth = 2
[[delegations]]
id = "delegation.global.scope-only"
issuer = "source.global"
recipient = "source.intermediate"
record_actions = ["delegation.issue", "role.define", "role.assign", "statement.allow"]
principals = ["principal.user"]
roles = ["role.project.*"]
operations = ["filesystem.read"]
scopes = ["path:src/**"]
redelegation = true
max_depth = 2
"#,
    )
    .unwrap();
    let error = load_policy(&environment, &[&global, &intermediate, &project])
        .unwrap_err()
        .to_string();
    assert!(error.contains("source.unauthorized"));
}

#[test]
fn delegation_source_cycles_are_rejected_with_a_stable_reason() {
    let directory = tempdir().unwrap();
    let environment = directory.path().join("environment.toml");
    let first = directory.path().join("first.toml");
    let second = directory.path().join("second.toml");
    fs::write(
        &environment,
        r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.cycle"
[[sources]]
id = "source.first"
kind = "global-policy"
locator = "first.toml"
trust_mode = "origin"
record_actions = ["delegation.issue"]
operations = ["*"]
scope_universe = ["path"]
[[sources]]
id = "source.second"
kind = "project-policy"
locator = "second.toml"
trust_mode = "delegated"
operations = ["*"]
scope_universe = ["path"]
"#,
    )
    .unwrap();
    fs::write(
        &first,
        r#"schema = "terminal-policy/records/v1"
source = "source.first"
[[delegations]]
id = "delegation.first.second"
issuer = "source.first"
recipient = "source.second"
record_actions = ["delegation.issue"]
operations = ["*"]
scope_universe = ["path"]
redelegation = true
max_depth = 2
"#,
    )
    .unwrap();
    fs::write(
        &second,
        r#"schema = "terminal-policy/records/v1"
source = "source.second"
[[delegations]]
id = "delegation.second.first"
issuer = "source.second"
recipient = "source.first"
record_actions = ["delegation.issue"]
operations = ["*"]
scope_universe = ["path"]
redelegation = true
max_depth = 1
"#,
    )
    .unwrap();

    let error = load_policy(&environment, &[first, second])
        .unwrap_err()
        .to_string();
    assert!(error.contains("delegation.cycle:source.first"));
}
