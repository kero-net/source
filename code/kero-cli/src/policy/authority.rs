use super::model::*;
use super::scope::scopes_contain;

#[derive(Clone, Debug, Default)]
pub struct Authority {
    pub authorized: bool,
    pub chains: Vec<Vec<String>>,
    pub reason: &'static str,
}

#[derive(Clone, Copy)]
pub enum RecordRef<'a> {
    Principal(&'a Principal),
    Role(&'a Role),
    Statement(&'a Statement),
    Assignment(&'a Assignment),
    Delegation(&'a Delegation),
}

impl<'a> RecordRef<'a> {
    fn id(self) -> &'a str {
        match self {
            Self::Principal(v) => &v.id,
            Self::Role(v) => &v.id,
            Self::Statement(v) => &v.id,
            Self::Assignment(v) => &v.id,
            Self::Delegation(v) => &v.id,
        }
    }
    fn source(self) -> &'a str {
        match self {
            Self::Principal(v) => &v.source,
            Self::Role(v) => &v.source,
            Self::Statement(v) => &v.source,
            Self::Assignment(v) => &v.source,
            Self::Delegation(v) => &v.source,
        }
    }
    fn action(self) -> String {
        match self {
            Self::Principal(_) => "principal.define".into(),
            Self::Role(_) => "role.define".into(),
            Self::Statement(v) => format!("statement.{}", v.effect),
            Self::Assignment(_) => "role.assign".into(),
            Self::Delegation(_) => "delegation.issue".into(),
        }
    }
    fn operations(self) -> &'a [String] {
        match self {
            Self::Statement(v) => &v.operations,
            Self::Assignment(v) => &v.operations,
            Self::Delegation(v) => &v.operations,
            _ => &[],
        }
    }
    fn scopes(self) -> &'a [ScopeValue] {
        match self {
            Self::Statement(v) => &v.scopes,
            Self::Assignment(v) => &v.scopes,
            Self::Delegation(v) => &v.scopes,
            _ => &[],
        }
    }
    fn universe(self) -> &'a [String] {
        match self {
            Self::Delegation(v) => &v.scope_universe,
            _ => &[],
        }
    }
}

pub fn operation_matches(selector: &str, operation: &str) -> bool {
    selector == "*"
        || selector == operation
        || selector.strip_suffix(".*").is_some_and(|prefix| {
            operation == prefix || operation.starts_with(&format!("{prefix}."))
        })
}

fn operations_contain(outer: &[String], inner: &[String]) -> bool {
    inner.iter().all(|child| {
        outer.iter().any(|parent| {
            if child == "*" {
                parent == "*"
            } else if let Some(child_prefix) = child.strip_suffix(".*") {
                parent == "*"
                    || parent.strip_suffix(".*").is_some_and(|parent_prefix| {
                        child_prefix == parent_prefix
                            || child_prefix.starts_with(&format!("{parent_prefix}."))
                    })
            } else {
                operation_matches(parent, child)
            }
        })
    })
}

fn id_matches(selector: &str, value: &str) -> bool {
    selector == "*"
        || selector == value
        || selector
            .strip_suffix(".*")
            .is_some_and(|prefix| value == prefix || value.starts_with(&format!("{prefix}.")))
}

fn selector_contains(outer: &str, inner: &str) -> bool {
    if outer == "*" || outer == inner {
        return true;
    }
    let Some(parent) = outer.strip_suffix(".*") else {
        return false;
    };
    let child = inner.strip_suffix(".*").unwrap_or(inner);
    child == parent || child.starts_with(&format!("{parent}."))
}

fn envelope_contains(delegation: &Delegation, record: RecordRef<'_>) -> bool {
    if !delegation.record_actions.contains(&record.action()) {
        return false;
    }
    match record {
        RecordRef::Principal(value) if !delegation.principals.is_empty() => {
            if !delegation
                .principals
                .iter()
                .any(|selector| id_matches(selector, &value.id))
            {
                return false;
            }
        }
        RecordRef::Role(value) if !delegation.roles.is_empty() => {
            if !delegation
                .roles
                .iter()
                .any(|selector| id_matches(selector, &value.id))
            {
                return false;
            }
        }
        RecordRef::Statement(value) if !delegation.roles.is_empty() => {
            if !delegation
                .roles
                .iter()
                .any(|selector| id_matches(selector, &value.role))
            {
                return false;
            }
        }
        RecordRef::Assignment(value) => {
            if !delegation.principals.is_empty()
                && !delegation
                    .principals
                    .iter()
                    .any(|selector| id_matches(selector, &value.principal))
            {
                return false;
            }
            if !delegation.roles.is_empty()
                && !value.roles.iter().all(|role| {
                    delegation
                        .roles
                        .iter()
                        .any(|selector| id_matches(selector, role))
                })
            {
                return false;
            }
        }
        RecordRef::Delegation(value) => {
            if !value
                .record_actions
                .iter()
                .all(|action| delegation.record_actions.contains(action))
            {
                return false;
            }
            if !value.principals.is_empty()
                && (delegation.principals.is_empty()
                    || !value.principals.iter().all(|child| {
                        delegation
                            .principals
                            .iter()
                            .any(|parent| selector_contains(parent, child))
                    }))
            {
                return false;
            }
            if !value.roles.is_empty()
                && (delegation.roles.is_empty()
                    || !value.roles.iter().all(|child| {
                        delegation
                            .roles
                            .iter()
                            .any(|parent| selector_contains(parent, child))
                    }))
            {
                return false;
            }
        }
        _ => {}
    }
    operations_contain(&delegation.operations, record.operations())
        && scopes_contain(
            &delegation.scopes,
            &delegation.scope_universe,
            record.scopes(),
            record.universe(),
        ) == Some(true)
}

fn role_is_deny_only(policy: &PolicySet, role_id: &str, active: &mut Vec<String>) -> bool {
    if active.iter().any(|item| item == role_id) {
        return false;
    }
    let Some(role) = policy.roles.get(role_id) else {
        return false;
    };
    if policy
        .environment
        .sources
        .get(&role.source)
        .is_some_and(|source| source.trust_mode == "delegated")
    {
        return false;
    }
    if policy
        .statements
        .values()
        .any(|statement| statement.role == role_id && statement.effect != "deny")
    {
        return false;
    }
    active.push(role_id.into());
    let valid = role
        .inherits
        .iter()
        .all(|parent| role_is_deny_only(policy, parent, active));
    active.pop();
    valid
}

fn restrictive_record_allowed(policy: &PolicySet, record: RecordRef<'_>) -> bool {
    match record {
        RecordRef::Statement(value) => value.effect == "deny",
        RecordRef::Role(value) => role_is_deny_only(policy, &value.id, &mut Vec::new()),
        RecordRef::Assignment(value) => {
            !value.roles.is_empty()
                && value
                    .roles
                    .iter()
                    .all(|role| role_is_deny_only(policy, role, &mut Vec::new()))
        }
        _ => false,
    }
}

pub fn source_authority(policy: &PolicySet, record: RecordRef<'_>) -> Authority {
    source_authority_inner(policy, record, &[])
}

fn source_authority_inner(
    policy: &PolicySet,
    record: RecordRef<'_>,
    trail: &[String],
) -> Authority {
    let Some(source) = policy.environment.sources.get(record.source()) else {
        return Authority {
            reason: "source.unknown",
            ..Authority::default()
        };
    };
    let action = record.action();
    let inside = operations_contain(&source.operations, record.operations())
        && scopes_contain(
            &source.scopes,
            &source.scope_universe,
            record.scopes(),
            record.universe(),
        ) == Some(true);
    if source.trust_mode == "origin" && source.record_actions.contains(&action) && inside {
        return Authority {
            authorized: true,
            chains: vec![Vec::new()],
            reason: "source.origin",
        };
    }
    if source.trust_mode == "restrictive"
        && source.record_actions.contains(&action)
        && inside
        && restrictive_record_allowed(policy, record)
    {
        return Authority {
            authorized: true,
            chains: vec![Vec::new()],
            reason: "source.restrictive",
        };
    }
    if trail.iter().any(|item| item == record.id()) {
        return Authority {
            reason: "delegation.cycle",
            ..Authority::default()
        };
    }
    let mut next_trail = trail.to_vec();
    next_trail.push(record.id().into());
    let mut chains = Vec::new();
    for delegation in policy.delegations.values() {
        if delegation.revoked
            || delegation.recipient != source.id
            || delegation.issuer != delegation.source
            || !envelope_contains(delegation, record)
        {
            continue;
        }
        if let RecordRef::Delegation(child) = record {
            if !delegation.redelegation
                || delegation.max_depth < 1
                || child.max_depth >= delegation.max_depth
            {
                continue;
            }
        }
        let parent = source_authority_inner(policy, RecordRef::Delegation(delegation), &next_trail);
        if !parent.authorized {
            continue;
        }
        for mut chain in parent.chains {
            chain.push(delegation.id.clone());
            chains.push(chain);
        }
    }
    if chains.is_empty() {
        Authority {
            reason: "delegation.missing",
            ..Authority::default()
        }
    } else {
        chains.sort();
        chains.dedup();
        Authority {
            authorized: true,
            chains,
            reason: "source.delegated",
        }
    }
}
