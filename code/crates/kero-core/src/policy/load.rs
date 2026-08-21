use super::model::*;
use super::scope::{ScopeError, parse_scope};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyLoadError {
    #[error("policy.io: {0}")]
    Io(#[from] std::io::Error),
    #[error("policy.toml: {0}")]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Scope(#[from] ScopeError),
    #[error("policy.schema-unsupported: {0}")]
    Schema(String),
    #[error("policy.id-duplicate: {0}")]
    Duplicate(String),
    #[error("policy.value-invalid: {0}")]
    Invalid(String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvironmentFile {
    schema: String,
    environment_id: String,
    #[serde(default)]
    context_providers: Vec<String>,
    #[serde(default)]
    sources: Vec<RawSource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    id: String,
    kind: String,
    locator: String,
    trust_mode: String,
    #[serde(default)]
    record_actions: Vec<String>,
    #[serde(default)]
    operations: Vec<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    scope_universe: Vec<String>,
    digest: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordsFile {
    schema: String,
    source: String,
    #[serde(default)]
    principals: Vec<RawPrincipal>,
    #[serde(default)]
    roles: Vec<RawRole>,
    #[serde(default)]
    statements: Vec<RawStatement>,
    #[serde(default)]
    assignments: Vec<RawAssignment>,
    #[serde(default)]
    delegations: Vec<RawDelegation>,
}

#[derive(Deserialize)]
struct RawPrincipal {
    id: String,
    #[serde(flatten)]
    metadata: BTreeMap<String, toml::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRole {
    id: String,
    #[serde(default)]
    inherits: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCondition {
    provider: String,
    key: String,
    operator: String,
    value: toml::Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStatement {
    id: String,
    role: String,
    effect: String,
    operations: Vec<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    conditions: Vec<RawCondition>,
    not_before: Option<String>,
    not_after: Option<String>,
    #[serde(default)]
    revoked: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAssignment {
    id: String,
    principal: String,
    roles: Vec<String>,
    #[serde(default)]
    operations: Vec<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    conditions: Vec<RawCondition>,
    not_before: Option<String>,
    not_after: Option<String>,
    #[serde(default)]
    revoked: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDelegation {
    id: String,
    issuer: String,
    recipient: String,
    record_actions: Vec<String>,
    #[serde(default)]
    principals: Vec<String>,
    #[serde(default)]
    roles: Vec<String>,
    #[serde(default)]
    operations: Vec<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    scope_universe: Vec<String>,
    #[serde(default)]
    conditions: Vec<RawCondition>,
    #[serde(default)]
    redelegation: bool,
    #[serde(default)]
    max_depth: u32,
    not_before: Option<String>,
    not_after: Option<String>,
    #[serde(default)]
    revoked: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestFile {
    schema: String,
    request_id: String,
    principal: String,
    operation: String,
    targets: Vec<String>,
    #[serde(default)]
    session: BTreeMap<String, toml::Value>,
    #[serde(default)]
    context: Vec<RawContext>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContext {
    provider: String,
    key: String,
    value: toml::Value,
    provenance: String,
    digest: String,
}

fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, PolicyLoadError> {
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

fn scopes(raw: Vec<String>) -> Result<Vec<ScopeValue>, PolicyLoadError> {
    raw.iter()
        .map(|value| parse_scope(value).map_err(PolicyLoadError::from))
        .collect()
}

fn json(value: toml::Value) -> Result<Value, PolicyLoadError> {
    serde_json::to_value(value).map_err(|error| PolicyLoadError::Invalid(error.to_string()))
}

fn conditions(raw: Vec<RawCondition>) -> Result<Vec<Condition>, PolicyLoadError> {
    raw.into_iter()
        .map(|item| {
            Ok(Condition {
                provider: item.provider,
                key: item.key,
                operator: item.operator,
                value: json(item.value)?,
            })
        })
        .collect()
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
}

fn require_id(kind: &str, value: &str) -> Result<(), PolicyLoadError> {
    if valid_id(value) {
        Ok(())
    } else {
        Err(PolicyLoadError::Invalid(format!(
            "id.malformed:{kind}:{value}"
        )))
    }
}

fn valid_operation(value: &str) -> bool {
    if value == "*" {
        return true;
    }
    let base = value.strip_suffix(".*").unwrap_or(value);
    !base.is_empty()
        && !base.contains('*')
        && base.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_lowercase())
                && segment.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
        })
}

fn validate_operations(record_id: &str, values: &[String]) -> Result<(), PolicyLoadError> {
    if values.is_empty() || values.iter().any(|value| !valid_operation(value)) {
        return Err(PolicyLoadError::Invalid(format!(
            "operation.selector-invalid:{record_id}"
        )));
    }
    Ok(())
}

fn validate_universe(record_id: &str, values: &[String]) -> Result<(), PolicyLoadError> {
    if values.iter().any(|value| !super::scope::type_known(value)) {
        return Err(PolicyLoadError::Invalid(format!(
            "scope.universe-type-unknown:{record_id}"
        )));
    }
    Ok(())
}

fn validate_conditions(
    policy: &PolicySet,
    record_id: &str,
    values: &[Condition],
) -> Result<(), PolicyLoadError> {
    for condition in values {
        if !policy
            .environment
            .context_providers
            .contains(&condition.provider)
        {
            return Err(PolicyLoadError::Invalid(format!(
                "condition.provider-unknown:{record_id}:{}",
                condition.provider
            )));
        }
        if !matches!(condition.operator.as_str(), "eq" | "ne" | "in" | "not-in") {
            return Err(PolicyLoadError::Invalid(format!(
                "condition.operator-unsupported:{record_id}:{}",
                condition.operator
            )));
        }
        if matches!(condition.operator.as_str(), "in" | "not-in") && !condition.value.is_array() {
            return Err(PolicyLoadError::Invalid(format!(
                "condition.value-invalid:{record_id}:{}",
                condition.key
            )));
        }
    }
    Ok(())
}

fn validate_window(
    record_id: &str,
    not_before: Option<&str>,
    not_after: Option<&str>,
) -> Result<(), PolicyLoadError> {
    let start = not_before
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| {
            PolicyLoadError::Invalid(format!("validity.malformed:{record_id}:not_before"))
        })?;
    let end = not_after
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| {
            PolicyLoadError::Invalid(format!("validity.malformed:{record_id}:not_after"))
        })?;
    if start.zip(end).is_some_and(|(start, end)| start > end) {
        return Err(PolicyLoadError::Invalid(format!(
            "validity.inverted:{record_id}"
        )));
    }
    Ok(())
}

fn validate_role_graph(policy: &PolicySet) -> Result<(), PolicyLoadError> {
    fn visit(
        policy: &PolicySet,
        role: &str,
        active: &mut Vec<String>,
        complete: &mut BTreeSet<String>,
    ) -> Result<(), PolicyLoadError> {
        if active.iter().any(|item| item == role) {
            return Err(PolicyLoadError::Invalid(format!(
                "role.inheritance-cycle:{role}"
            )));
        }
        if complete.contains(role) {
            return Ok(());
        }
        active.push(role.into());
        for parent in &policy.roles[role].inherits {
            if !policy.roles.contains_key(parent) {
                return Err(PolicyLoadError::Invalid(format!(
                    "role.inherited-missing:{role}:{parent}"
                )));
            }
            visit(policy, parent, active, complete)?;
        }
        active.pop();
        complete.insert(role.into());
        Ok(())
    }

    let mut complete = BTreeSet::new();
    for role in policy.roles.keys() {
        visit(policy, role, &mut Vec::new(), &mut complete)?;
    }
    Ok(())
}

fn validate_delegation_graph(policy: &PolicySet) -> Result<(), PolicyLoadError> {
    fn visit(
        policy: &PolicySet,
        source: &str,
        active: &mut Vec<String>,
        complete: &mut BTreeSet<String>,
    ) -> Result<(), PolicyLoadError> {
        if active.iter().any(|item| item == source) {
            return Err(PolicyLoadError::Invalid(format!(
                "delegation.cycle:{source}"
            )));
        }
        if complete.contains(source) {
            return Ok(());
        }
        active.push(source.into());
        for recipient in policy
            .delegations
            .values()
            .filter(|delegation| delegation.issuer == source)
            .map(|delegation| delegation.recipient.as_str())
        {
            visit(policy, recipient, active, complete)?;
        }
        active.pop();
        complete.insert(source.into());
        Ok(())
    }

    let mut complete = BTreeSet::new();
    for source in policy.environment.sources.keys() {
        visit(policy, source, &mut Vec::new(), &mut complete)?;
    }
    Ok(())
}

pub(super) fn validate_policy(policy: &PolicySet) -> Result<(), PolicyLoadError> {
    const RECORD_ACTIONS: &[&str] = &[
        "principal.define",
        "role.define",
        "role.assign",
        "statement.allow",
        "statement.deny",
        "delegation.issue",
    ];
    require_id("environment", &policy.environment.environment_id)?;
    for source in policy.environment.sources.values() {
        require_id("source", &source.id)?;
        if !matches!(
            source.trust_mode.as_str(),
            "origin" | "delegated" | "restrictive"
        ) {
            return Err(PolicyLoadError::Invalid(format!(
                "source.trust-mode-unsupported:{}:{}",
                source.id, source.trust_mode
            )));
        }
        if source.locator.is_empty() {
            return Err(PolicyLoadError::Invalid(format!(
                "source.incomplete:{}",
                source.id
            )));
        }
        validate_operations(&source.id, &source.operations)?;
        validate_universe(&source.id, &source.scope_universe)?;
        if source
            .record_actions
            .iter()
            .any(|action| !RECORD_ACTIONS.contains(&action.as_str()))
        {
            return Err(PolicyLoadError::Invalid(format!(
                "source.record-action-unsupported:{}",
                source.id
            )));
        }
    }
    let mut record_ids = BTreeSet::new();
    for principal in policy.principals.values() {
        require_id("principal", &principal.id)?;
        if !record_ids.insert(&principal.id) {
            return Err(PolicyLoadError::Duplicate(principal.id.clone()));
        }
    }
    for role in policy.roles.values() {
        require_id("role", &role.id)?;
        if !record_ids.insert(&role.id) {
            return Err(PolicyLoadError::Duplicate(role.id.clone()));
        }
    }
    validate_role_graph(policy)?;
    for statement in policy.statements.values() {
        require_id("statement", &statement.id)?;
        if !record_ids.insert(&statement.id) {
            return Err(PolicyLoadError::Duplicate(statement.id.clone()));
        }
        if !policy.roles.contains_key(&statement.role) {
            return Err(PolicyLoadError::Invalid(format!(
                "statement.role-missing:{}:{}",
                statement.id, statement.role
            )));
        }
        if !matches!(statement.effect.as_str(), "allow" | "deny") {
            return Err(PolicyLoadError::Invalid(format!(
                "statement.effect-unsupported:{}:{}",
                statement.id, statement.effect
            )));
        }
        validate_operations(&statement.id, &statement.operations)?;
        validate_conditions(policy, &statement.id, &statement.conditions)?;
        validate_window(
            &statement.id,
            statement.not_before.as_deref(),
            statement.not_after.as_deref(),
        )?;
    }
    for assignment in policy.assignments.values() {
        require_id("assignment", &assignment.id)?;
        if !record_ids.insert(&assignment.id) {
            return Err(PolicyLoadError::Duplicate(assignment.id.clone()));
        }
        if !policy.principals.contains_key(&assignment.principal) {
            return Err(PolicyLoadError::Invalid(format!(
                "assignment.principal-missing:{}:{}",
                assignment.id, assignment.principal
            )));
        }
        if assignment.roles.is_empty() {
            return Err(PolicyLoadError::Invalid(format!(
                "assignment.roles-empty:{}",
                assignment.id
            )));
        }
        for role in &assignment.roles {
            if !policy.roles.contains_key(role) {
                return Err(PolicyLoadError::Invalid(format!(
                    "assignment.role-missing:{}:{role}",
                    assignment.id
                )));
            }
        }
        validate_conditions(policy, &assignment.id, &assignment.conditions)?;
        if !assignment.operations.is_empty() {
            validate_operations(&assignment.id, &assignment.operations)?;
        }
        validate_window(
            &assignment.id,
            assignment.not_before.as_deref(),
            assignment.not_after.as_deref(),
        )?;
    }
    for delegation in policy.delegations.values() {
        require_id("delegation", &delegation.id)?;
        if !record_ids.insert(&delegation.id) {
            return Err(PolicyLoadError::Duplicate(delegation.id.clone()));
        }
        if delegation.issuer != delegation.source
            || !policy
                .environment
                .sources
                .contains_key(&delegation.recipient)
        {
            return Err(PolicyLoadError::Invalid(format!(
                "delegation.link-invalid:{}",
                delegation.id
            )));
        }
        if delegation.record_actions.is_empty() {
            return Err(PolicyLoadError::Invalid(format!(
                "delegation.envelope-empty:{}",
                delegation.id
            )));
        }
        validate_operations(&delegation.id, &delegation.operations)?;
        validate_universe(&delegation.id, &delegation.scope_universe)?;
        if delegation
            .record_actions
            .iter()
            .any(|action| !RECORD_ACTIONS.contains(&action.as_str()))
        {
            return Err(PolicyLoadError::Invalid(format!(
                "delegation.record-action-unsupported:{}",
                delegation.id
            )));
        }
        validate_conditions(policy, &delegation.id, &delegation.conditions)?;
        validate_window(
            &delegation.id,
            delegation.not_before.as_deref(),
            delegation.not_after.as_deref(),
        )?;
    }
    validate_delegation_graph(policy)?;

    use super::authority::{RecordRef, source_authority};
    let records = policy
        .principals
        .values()
        .map(RecordRef::Principal)
        .chain(policy.roles.values().map(RecordRef::Role))
        .chain(policy.statements.values().map(RecordRef::Statement))
        .chain(policy.assignments.values().map(RecordRef::Assignment))
        .chain(policy.delegations.values().map(RecordRef::Delegation));
    for record in records {
        let id = match record {
            RecordRef::Principal(value) => &value.id,
            RecordRef::Role(value) => &value.id,
            RecordRef::Statement(value) => &value.id,
            RecordRef::Assignment(value) => &value.id,
            RecordRef::Delegation(value) => &value.id,
        };
        let authority = source_authority(policy, record);
        if !authority.authorized {
            return Err(PolicyLoadError::Invalid(format!(
                "source.unauthorized:{id}:{}",
                authority.reason
            )));
        }
    }
    Ok(())
}

pub fn load_environment(path: &Path) -> Result<PolicyEnvironment, PolicyLoadError> {
    let raw: EnvironmentFile = read(path)?;
    if raw.schema != ENVIRONMENT_SCHEMA {
        return Err(PolicyLoadError::Schema(raw.schema));
    }
    let mut sources = BTreeMap::new();
    for item in raw.sources {
        let id = item.id.clone();
        let source = PolicySource {
            id: id.clone(),
            kind: item.kind,
            locator: item.locator,
            trust_mode: item.trust_mode,
            record_actions: item.record_actions,
            operations: item.operations,
            scopes: scopes(item.scopes)?,
            scope_universe: item.scope_universe,
            digest: item.digest,
        };
        if sources.insert(id.clone(), source).is_some() {
            return Err(PolicyLoadError::Duplicate(id));
        }
    }
    let context_providers = if raw.context_providers.is_empty() {
        ["branch", "environment", "project", "revocation", "time"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    } else {
        raw.context_providers.into_iter().collect()
    };
    Ok(PolicyEnvironment {
        schema: raw.schema,
        environment_id: raw.environment_id,
        sources,
        context_providers,
    })
}

fn insert<T>(map: &mut BTreeMap<String, T>, id: String, value: T) -> Result<(), PolicyLoadError> {
    if map.insert(id.clone(), value).is_some() {
        return Err(PolicyLoadError::Duplicate(id));
    }
    Ok(())
}

pub fn load_policy(
    environment: &Path,
    records: &[impl AsRef<Path>],
) -> Result<PolicySet, PolicyLoadError> {
    let mut policy = PolicySet {
        environment: load_environment(environment)?,
        principals: BTreeMap::new(),
        roles: BTreeMap::new(),
        statements: BTreeMap::new(),
        assignments: BTreeMap::new(),
        delegations: BTreeMap::new(),
        diagnostics: Vec::new(),
    };
    for path in records {
        let raw: RecordsFile = read(path.as_ref())?;
        if raw.schema != RECORDS_SCHEMA {
            return Err(PolicyLoadError::Schema(raw.schema));
        }
        if !policy.environment.sources.contains_key(&raw.source) {
            return Err(PolicyLoadError::Invalid(format!(
                "unknown source: {}",
                raw.source
            )));
        }
        for item in raw.principals {
            let metadata = item
                .metadata
                .into_iter()
                .map(|(key, value)| Ok((key, json(value)?)))
                .collect::<Result<_, PolicyLoadError>>()?;
            let id = item.id;
            insert(
                &mut policy.principals,
                id.clone(),
                Principal {
                    id,
                    source: raw.source.clone(),
                    metadata,
                },
            )?;
        }
        for item in raw.roles {
            let id = item.id;
            insert(
                &mut policy.roles,
                id.clone(),
                Role {
                    id,
                    source: raw.source.clone(),
                    inherits: item.inherits,
                },
            )?;
        }
        for item in raw.statements {
            let id = item.id;
            let value = Statement {
                id: id.clone(),
                source: raw.source.clone(),
                role: item.role,
                effect: item.effect,
                operations: item.operations,
                scopes: scopes(item.scopes)?,
                conditions: conditions(item.conditions)?,
                not_before: item.not_before,
                not_after: item.not_after,
                revoked: item.revoked,
            };
            insert(&mut policy.statements, id, value)?;
        }
        for item in raw.assignments {
            let id = item.id;
            let value = Assignment {
                id: id.clone(),
                source: raw.source.clone(),
                principal: item.principal,
                roles: item.roles,
                operations: item.operations,
                scopes: scopes(item.scopes)?,
                conditions: conditions(item.conditions)?,
                not_before: item.not_before,
                not_after: item.not_after,
                revoked: item.revoked,
            };
            insert(&mut policy.assignments, id, value)?;
        }
        for item in raw.delegations {
            let id = item.id;
            let value = Delegation {
                id: id.clone(),
                source: raw.source.clone(),
                issuer: item.issuer,
                recipient: item.recipient,
                record_actions: item.record_actions,
                principals: item.principals,
                roles: item.roles,
                operations: item.operations,
                scopes: scopes(item.scopes)?,
                scope_universe: item.scope_universe,
                conditions: conditions(item.conditions)?,
                redelegation: item.redelegation,
                max_depth: item.max_depth,
                not_before: item.not_before,
                not_after: item.not_after,
                revoked: item.revoked,
            };
            insert(&mut policy.delegations, id, value)?;
        }
    }
    validate_policy(&policy)?;
    Ok(policy)
}

pub fn load_request(path: &Path) -> Result<AuthorizationRequest, PolicyLoadError> {
    let raw: RequestFile = read(path)?;
    if raw.schema != REQUEST_SCHEMA {
        return Err(PolicyLoadError::Schema(raw.schema));
    }
    let session = raw
        .session
        .into_iter()
        .map(|(key, value)| Ok((key, json(value)?)))
        .collect::<Result<_, PolicyLoadError>>()?;
    require_id("request", &raw.request_id)?;
    require_id("principal", &raw.principal)?;
    if raw.operation.is_empty() || raw.targets.is_empty() {
        return Err(PolicyLoadError::Invalid("request.incomplete".into()));
    }
    if !valid_operation(&raw.operation) || raw.operation == "*" || raw.operation.ends_with(".*") {
        return Err(PolicyLoadError::Invalid(format!(
            "request.operation-invalid:{}",
            raw.operation
        )));
    }
    let context = raw
        .context
        .into_iter()
        .map(|item| {
            Ok(ContextEvidence {
                provider: item.provider,
                key: item.key,
                value: json(item.value)?,
                provenance: item.provenance,
                digest: item.digest,
            })
        })
        .collect::<Result<Vec<_>, PolicyLoadError>>()?;
    let mut context_keys = BTreeSet::new();
    for item in &context {
        if item.provider.is_empty()
            || item.key.is_empty()
            || item.provenance.is_empty()
            || item.digest.is_empty()
        {
            return Err(PolicyLoadError::Invalid(format!(
                "context.incomplete:{}:{}",
                item.provider, item.key
            )));
        }
        if !context_keys.insert((&item.provider, &item.key)) {
            return Err(PolicyLoadError::Invalid(format!(
                "context.duplicate:{}:{}",
                item.provider, item.key
            )));
        }
    }
    let targets = scopes(raw.targets)?;
    let mut target_values = BTreeSet::new();
    if targets
        .iter()
        .any(|target| !target_values.insert(target.canonical()))
    {
        return Err(PolicyLoadError::Invalid("request.targets-duplicate".into()));
    }
    Ok(AuthorizationRequest {
        schema: raw.schema,
        request_id: raw.request_id,
        principal: raw.principal,
        operation: raw.operation,
        targets,
        session,
        context,
    })
}
