use super::model::*;
use super::scope::{ScopeError, parse_scope};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
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
struct EnvironmentFile {
    schema: String,
    environment_id: String,
    #[serde(default)]
    context_providers: Vec<String>,
    #[serde(default)]
    sources: Vec<RawSource>,
}

#[derive(Deserialize)]
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
struct RawRole {
    id: String,
    #[serde(default)]
    inherits: Vec<String>,
}

#[derive(Deserialize)]
struct RawCondition {
    provider: String,
    key: String,
    operator: String,
    value: toml::Value,
}

#[derive(Deserialize)]
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
        .collect::<Result<_, PolicyLoadError>>()?;
    Ok(AuthorizationRequest {
        schema: raw.schema,
        request_id: raw.request_id,
        principal: raw.principal,
        operation: raw.operation,
        targets: scopes(raw.targets)?,
        session,
        context,
    })
}
