use super::authority::{RecordRef, operation_matches, source_authority};
use super::model::*;
use super::scope::scope_set_match;
use super::snapshot::{SnapshotError, write_snapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
}

fn combine(states: impl IntoIterator<Item = Applicability>) -> Applicability {
    let states: Vec<_> = states.into_iter().collect();
    if states.contains(&Applicability::Inapplicable) {
        Applicability::Inapplicable
    } else if states.contains(&Applicability::Indeterminate) {
        Applicability::Indeterminate
    } else {
        Applicability::Applicable
    }
}

fn role_paths(policy: &PolicySet, start: &str) -> BTreeMap<String, Vec<Vec<String>>> {
    fn visit(
        policy: &PolicySet,
        role_id: &str,
        path: &[String],
        result: &mut BTreeMap<String, Vec<Vec<String>>>,
    ) {
        if path.iter().any(|item| item == role_id) {
            return;
        }
        let mut next = path.to_vec();
        next.push(role_id.into());
        result.entry(role_id.into()).or_default().push(next.clone());
        if let Some(role) = policy.roles.get(role_id) {
            for parent in &role.inherits {
                visit(policy, parent, &next, result);
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(policy, start, &[], &mut result);
    result
}

fn context<'a>(
    request: &'a AuthorizationRequest,
    provider: &str,
    key: &str,
) -> Option<&'a ContextEvidence> {
    request.context.iter().find(|item| {
        item.provider == provider
            && item.key == key
            && !item.provenance.is_empty()
            && !item.digest.is_empty()
    })
}

fn conditions_state(conditions: &[Condition], request: &AuthorizationRequest) -> Applicability {
    combine(conditions.iter().map(|condition| {
        let Some(evidence) = context(request, &condition.provider, &condition.key) else {
            return Applicability::Indeterminate;
        };
        let matched = match condition.operator.as_str() {
            "eq" => evidence.value == condition.value,
            "ne" => evidence.value != condition.value,
            "in" => condition
                .value
                .as_array()
                .is_some_and(|values| values.contains(&evidence.value)),
            "not-in" => condition
                .value
                .as_array()
                .is_some_and(|values| !values.contains(&evidence.value)),
            _ => false,
        };
        if matched {
            Applicability::Applicable
        } else {
            Applicability::Inapplicable
        }
    }))
}

fn validity_state(
    revoked: bool,
    not_before: Option<&str>,
    not_after: Option<&str>,
    request: &AuthorizationRequest,
) -> Applicability {
    if revoked {
        return Applicability::Inapplicable;
    }
    if not_before.is_none() && not_after.is_none() {
        return Applicability::Applicable;
    }
    let Some(now) = context(request, "time", "now")
        .and_then(|item| item.value.as_str())
        .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
    else {
        return Applicability::Indeterminate;
    };
    if not_before
        .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
        .is_some_and(|start| now < start)
        || not_after
            .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
            .is_some_and(|end| now > end)
    {
        Applicability::Inapplicable
    } else {
        Applicability::Applicable
    }
}

fn authority_state(
    policy: &PolicySet,
    record: RecordRef<'_>,
    request: &AuthorizationRequest,
) -> (Applicability, Vec<Vec<String>>, &'static str) {
    let authority = source_authority(policy, record);
    if !authority.authorized {
        return (
            Applicability::Inapplicable,
            authority.chains,
            authority.reason,
        );
    }
    let mut states = Vec::new();
    for chain in &authority.chains {
        let state = combine(
            chain
                .iter()
                .filter_map(|id| policy.delegations.get(id))
                .flat_map(|item| {
                    let operation = if item.operations.is_empty()
                        || item
                            .operations
                            .iter()
                            .any(|selector| operation_matches(selector, &request.operation))
                    {
                        Applicability::Applicable
                    } else {
                        Applicability::Inapplicable
                    };
                    let scope =
                        match scope_set_match(&item.scopes, &item.scope_universe, &request.targets)
                        {
                            Some(true) => Applicability::Applicable,
                            Some(false) => Applicability::Inapplicable,
                            None => Applicability::Indeterminate,
                        };
                    [
                        operation,
                        scope,
                        conditions_state(&item.conditions, request),
                        validity_state(
                            item.revoked,
                            item.not_before.as_deref(),
                            item.not_after.as_deref(),
                            request,
                        ),
                    ]
                }),
        );
        states.push(state);
    }
    let outcome = if states.contains(&Applicability::Applicable) {
        Applicability::Applicable
    } else if states.contains(&Applicability::Indeterminate) {
        Applicability::Indeterminate
    } else {
        Applicability::Inapplicable
    };
    (outcome, authority.chains, authority.reason)
}

fn source_request_state(
    policy: &PolicySet,
    source_id: &str,
    request: &AuthorizationRequest,
) -> Applicability {
    let Some(source) = policy.environment.sources.get(source_id) else {
        return Applicability::Inapplicable;
    };
    combine([
        if source
            .operations
            .iter()
            .any(|selector| operation_matches(selector, &request.operation))
        {
            Applicability::Applicable
        } else {
            Applicability::Inapplicable
        },
        match scope_set_match(&source.scopes, &source.scope_universe, &request.targets) {
            Some(true) => Applicability::Applicable,
            Some(false) => Applicability::Inapplicable,
            None => Applicability::Indeterminate,
        },
    ])
}

fn assignment_state(
    policy: &PolicySet,
    assignment: &Assignment,
    request: &AuthorizationRequest,
) -> AssignmentEvaluation {
    let mut reasons = Vec::new();
    let principal_authority = policy
        .principals
        .get(&assignment.principal)
        .map_or(Applicability::Inapplicable, |principal| {
            authority_state(policy, RecordRef::Principal(principal), request).0
        });
    let record_authority = authority_state(policy, RecordRef::Assignment(assignment), request).0;
    let operation = assignment.operations.is_empty()
        || assignment
            .operations
            .iter()
            .any(|item| operation_matches(item, &request.operation));
    let scope = match scope_set_match(&assignment.scopes, &[], &request.targets) {
        Some(true) => Applicability::Applicable,
        Some(false) => Applicability::Inapplicable,
        None => Applicability::Indeterminate,
    };
    if assignment.principal != request.principal {
        reasons.push("assignment.principal-mismatch".into());
    }
    if principal_authority == Applicability::Inapplicable {
        reasons.push("principal.unauthorized".into());
    }
    if record_authority == Applicability::Inapplicable {
        reasons.push("source.unauthorized".into());
    }
    if !operation {
        reasons.push("operation.mismatch".into());
    }
    let applicability = combine([
        if assignment.principal == request.principal {
            combine([principal_authority, record_authority])
        } else {
            Applicability::Inapplicable
        },
        if operation {
            Applicability::Applicable
        } else {
            Applicability::Inapplicable
        },
        source_request_state(policy, &assignment.source, request),
        scope,
        conditions_state(&assignment.conditions, request),
        validity_state(
            assignment.revoked,
            assignment.not_before.as_deref(),
            assignment.not_after.as_deref(),
            request,
        ),
    ]);
    if applicability == Applicability::Indeterminate {
        reasons.push("assignment.indeterminate".into());
    }
    AssignmentEvaluation {
        id: assignment.id.clone(),
        applicability,
        reasons,
    }
}

fn cycle_diagnostics(policy: &PolicySet) -> Vec<Diagnostic> {
    fn visit(
        policy: &PolicySet,
        role: &str,
        active: &mut Vec<String>,
        visited: &mut BTreeSet<String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if let Some(index) = active.iter().position(|item| item == role) {
            let mut cycle = active[index..].to_vec();
            cycle.push(role.into());
            diagnostics.push(Diagnostic {
                code: "role.inheritance-cycle".into(),
                severity: "error".into(),
                message: format!("Role inheritance cycle: {}", cycle.join(" -> ")),
                record_id: Some(role.into()),
            });
            return;
        }
        if !visited.insert(role.into()) {
            return;
        }
        active.push(role.into());
        if let Some(value) = policy.roles.get(role) {
            for parent in &value.inherits {
                visit(policy, parent, active, visited, diagnostics);
            }
        }
        active.pop();
    }
    let mut diagnostics = Vec::new();
    let mut visited = BTreeSet::new();
    for role in policy.roles.keys() {
        visit(
            policy,
            role,
            &mut Vec::new(),
            &mut visited,
            &mut diagnostics,
        );
    }
    diagnostics
}

pub fn authorize(
    policy: &PolicySet,
    request: &AuthorizationRequest,
    snapshot_store: &Path,
) -> Result<PureAuthorizationResult, AuthorizationError> {
    let mut diagnostics = policy.diagnostics.clone();
    diagnostics.extend(cycle_diagnostics(policy));
    if request.session.contains_key("roles") {
        diagnostics.push(Diagnostic {
            code: "session.roles-ignored".into(),
            severity: "warning".into(),
            message: "Session-declared roles are not authoritative".into(),
            record_id: None,
        });
    }
    let (snapshot_id, snapshot_digest, snapshot_path) = write_snapshot(policy, snapshot_store)?;
    let assignments: Vec<_> = policy
        .assignments
        .values()
        .map(|item| assignment_state(policy, item, request))
        .collect();
    let mut reached: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    let mut reached_chains: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    let mut indeterminate_roles = BTreeSet::new();
    for (assignment, evaluation) in policy.assignments.values().zip(&assignments) {
        if evaluation.applicability == Applicability::Inapplicable {
            continue;
        }
        for role in &assignment.roles {
            for (reached_role, paths) in role_paths(policy, role) {
                for path in paths {
                    let assignment_authority =
                        authority_state(policy, RecordRef::Assignment(assignment), request);
                    let role_authorities: Vec<_> = path
                        .iter()
                        .filter_map(|item| policy.roles.get(item))
                        .map(|role| authority_state(policy, RecordRef::Role(role), request))
                        .collect();
                    if assignment_authority.0 != Applicability::Inapplicable
                        && role_authorities
                            .iter()
                            .all(|item| item.0 != Applicability::Inapplicable)
                    {
                        let mut complete = vec![assignment.id.clone()];
                        complete.extend(path);
                        reached
                            .entry(reached_role.clone())
                            .or_default()
                            .push(complete);
                        let chains = reached_chains.entry(reached_role.clone()).or_default();
                        chains.extend(assignment_authority.1);
                        for authority in &role_authorities {
                            chains.extend(authority.1.clone());
                        }
                        chains.sort();
                        chains.dedup();
                        if evaluation.applicability == Applicability::Indeterminate
                            || assignment_authority.0 == Applicability::Indeterminate
                            || role_authorities
                                .iter()
                                .any(|item| item.0 == Applicability::Indeterminate)
                        {
                            indeterminate_roles.insert(reached_role.clone());
                        }
                    }
                }
            }
        }
    }
    let statements: Vec<_> = policy
        .statements
        .values()
        .map(|statement| {
            let statement_authority =
                authority_state(policy, RecordRef::Statement(statement), request);
            let paths = reached.get(&statement.role).cloned().unwrap_or_default();
            let assignment = if paths.is_empty() {
                Applicability::Inapplicable
            } else if indeterminate_roles.contains(&statement.role) {
                Applicability::Indeterminate
            } else {
                Applicability::Applicable
            };
            let operation = if statement
                .operations
                .iter()
                .any(|item| operation_matches(item, &request.operation))
            {
                Applicability::Applicable
            } else {
                Applicability::Inapplicable
            };
            let scope = match scope_set_match(&statement.scopes, &[], &request.targets) {
                Some(true) => Applicability::Applicable,
                Some(false) => Applicability::Inapplicable,
                None => Applicability::Indeterminate,
            };
            let conditions = conditions_state(&statement.conditions, request);
            let applicability = combine([
                statement_authority.0,
                assignment,
                operation,
                scope,
                conditions,
                source_request_state(policy, &statement.source, request),
                validity_state(
                    statement.revoked,
                    statement.not_before.as_deref(),
                    statement.not_after.as_deref(),
                    request,
                ),
            ]);
            let mut reasons = Vec::new();
            if statement_authority.0 == Applicability::Inapplicable {
                reasons.push(statement_authority.2.into());
            } else if statement_authority.0 == Applicability::Indeterminate {
                reasons.push("delegation.indeterminate".into());
            }
            if paths.is_empty() {
                reasons.push("role.unreachable".into());
            }
            if operation == Applicability::Inapplicable {
                reasons.push("operation.mismatch".into());
            }
            if scope == Applicability::Indeterminate {
                reasons.push("scope.indeterminate".into());
            } else if scope == Applicability::Inapplicable {
                reasons.push("scope.mismatch".into());
            }
            if conditions == Applicability::Indeterminate {
                reasons.push("condition.indeterminate".into());
            } else if conditions == Applicability::Inapplicable {
                reasons.push("condition.unsatisfied".into());
            }
            StatementEvaluation {
                id: statement.id.clone(),
                effect: statement.effect.clone(),
                source: statement.source.clone(),
                applicability,
                reasons,
                assignment_paths: paths,
                delegation_chains: {
                    let mut chains = statement_authority.1;
                    chains.extend(
                        reached_chains
                            .get(&statement.role)
                            .cloned()
                            .unwrap_or_default(),
                    );
                    chains.sort();
                    chains.dedup();
                    chains
                },
            }
        })
        .collect();
    let explicit_deny = statements
        .iter()
        .any(|item| item.effect == "deny" && item.applicability == Applicability::Applicable);
    let uncertain_deny = statements
        .iter()
        .any(|item| item.effect == "deny" && item.applicability == Applicability::Indeterminate);
    let allow = statements
        .iter()
        .any(|item| item.effect == "allow" && item.applicability == Applicability::Applicable);
    let (decision, reason) = if diagnostics.iter().any(|item| item.severity == "error") {
        ("DENY", "policy.invalid")
    } else if explicit_deny {
        ("DENY", "deny.explicit")
    } else if uncertain_deny {
        ("DENY", "deny.policy-indeterminate")
    } else if allow {
        ("ALLOW", "allow.applicable")
    } else {
        ("DENY", "deny.default")
    };
    let effective_roles = reached
        .keys()
        .filter(|role| !indeterminate_roles.contains(*role))
        .cloned()
        .collect();
    Ok(PureAuthorizationResult {
        schema: "terminal-policy/authorization-result/v1".into(),
        request_id: request.request_id.clone(),
        snapshot: SnapshotBinding {
            id: snapshot_id,
            digest: snapshot_digest,
            retrieval: format!("file:{}", snapshot_path.display()),
            schema: SNAPSHOT_SCHEMA.into(),
            resolver_version: RESOLVER_VERSION.into(),
        },
        principal: request.principal.clone(),
        operation: request.operation.clone(),
        targets: request.targets.iter().map(ScopeValue::canonical).collect(),
        decision: decision.into(),
        reason: reason.into(),
        effective_roles,
        indeterminate_roles: indeterminate_roles.into_iter().collect(),
        assignments,
        statements,
        diagnostics,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::policy::{load_policy, load_request};
    use std::fs;
    use tempfile::tempdir;

    pub(crate) const ENVIRONMENT: &str = r#"schema = "terminal-policy/environment/v1"
environment_id = "environment.test"
context_providers = ["branch", "time"]
[[sources]]
id = "source.global"
kind = "global-policy"
locator = "records.toml"
trust_mode = "origin"
record_actions = ["principal.define", "role.define", "role.assign", "statement.allow", "statement.deny", "delegation.issue"]
operations = ["*"]
scope_universe = ["path", "repository"]
"#;
    pub(crate) const RECORDS: &str = r#"schema = "terminal-policy/records/v1"
source = "source.global"
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
"#;
    const REQUEST: &str = r#"schema = "terminal-policy/request/v1"
request_id = "request.test"
principal = "principal.user"
operation = "filesystem.write"
targets = ["path:src/main.rs"]
[session]
id = "session.test"
"#;

    fn fixture(extra: &str, request_extra: &str) -> PureAuthorizationResult {
        let directory = tempdir().unwrap();
        let environment = directory.path().join("environment.toml");
        let records = directory.path().join("records.toml");
        let request = directory.path().join("request.toml");
        fs::write(&environment, ENVIRONMENT).unwrap();
        fs::write(&records, format!("{RECORDS}{extra}")).unwrap();
        fs::write(&request, format!("{REQUEST}{request_extra}")).unwrap();
        authorize(
            &load_policy(&environment, &[&records]).unwrap(),
            &load_request(&request).unwrap(),
            &directory.path().join("snapshots"),
        )
        .unwrap()
    }

    #[test]
    fn applicable_allow_and_explicit_deny_resolve_deterministically() {
        assert_eq!(fixture("", "").decision, "ALLOW");
        let result = fixture(
            r#"
[[statements]]
id = "statement.deny-write"
role = "role.contributor"
effect = "deny"
operations = ["filesystem.write"]
scopes = ["path:src/**"]
"#,
            "",
        );
        assert_eq!(
            (result.decision.as_str(), result.reason.as_str()),
            ("DENY", "deny.explicit")
        );
    }

    #[test]
    fn indeterminate_deny_fails_closed_but_unknown_allow_cannot_grant() {
        let deny = fixture(
            r#"
[[statements]]
id = "statement.protect-main"
role = "role.contributor"
effect = "deny"
operations = ["filesystem.write"]
scopes = ["path:src/**"]
conditions = [{ provider = "branch", key = "name", operator = "eq", value = "main" }]
"#,
            "",
        );
        assert_eq!(
            (deny.decision.as_str(), deny.reason.as_str()),
            ("DENY", "deny.policy-indeterminate")
        );
        let allow = fixture("", "")
            .statements
            .into_iter()
            .find(|item| item.effect == "allow")
            .unwrap();
        assert_eq!(allow.applicability, Applicability::Applicable);
    }

    #[test]
    fn session_role_claims_never_create_authority() {
        let result = fixture("", "\nroles = [\"role.root\"]\n");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "session.roles-ignored")
        );
        assert!(
            !result
                .effective_roles
                .iter()
                .any(|item| item == "role.root")
        );
    }
}
