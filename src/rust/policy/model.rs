use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub const ENVIRONMENT_SCHEMA: &str = "terminal-policy/environment/v1";
pub const RECORDS_SCHEMA: &str = "terminal-policy/records/v1";
pub const REQUEST_SCHEMA: &str = "terminal-policy/request/v1";
pub const SNAPSHOT_SCHEMA: &str = "terminal-policy/snapshot/v1";
pub const RESOLVER_VERSION: &str = "terminal-policy-resolver/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Applicability {
    Applicable,
    Inapplicable,
    Indeterminate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ScopeValue {
    #[serde(rename = "type")]
    pub kind: String,
    pub expression: String,
}

impl ScopeValue {
    pub fn canonical(&self) -> String {
        format!("{}:{}", self.kind, self.expression)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Condition {
    pub provider: String,
    pub key: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ContextEvidence {
    pub provider: String,
    pub key: String,
    pub value: Value,
    pub provenance: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolicySource {
    pub id: String,
    pub kind: String,
    pub locator: String,
    pub trust_mode: String,
    #[serde(default)]
    pub record_actions: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<ScopeValue>,
    #[serde(default)]
    pub scope_universe: Vec<String>,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolicyEnvironment {
    pub schema: String,
    pub environment_id: String,
    pub sources: BTreeMap<String, PolicySource>,
    pub context_providers: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Principal {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Role {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub inherits: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Statement {
    pub id: String,
    pub source: String,
    pub role: String,
    pub effect: String,
    pub operations: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<ScopeValue>,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Assignment {
    pub id: String,
    pub source: String,
    pub principal: String,
    pub roles: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<ScopeValue>,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Delegation {
    pub id: String,
    pub source: String,
    pub issuer: String,
    pub recipient: String,
    pub record_actions: Vec<String>,
    #[serde(default)]
    pub principals: Vec<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub scopes: Vec<ScopeValue>,
    #[serde(default)]
    pub scope_universe: Vec<String>,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub redelegation: bool,
    #[serde(default)]
    pub max_depth: u32,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    #[serde(default)]
    pub revoked: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub record_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PolicySet {
    pub environment: PolicyEnvironment,
    pub principals: BTreeMap<String, Principal>,
    pub roles: BTreeMap<String, Role>,
    pub statements: BTreeMap<String, Statement>,
    pub assignments: BTreeMap<String, Assignment>,
    pub delegations: BTreeMap<String, Delegation>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AuthorizationRequest {
    pub schema: String,
    pub request_id: String,
    pub principal: String,
    pub operation: String,
    pub targets: Vec<ScopeValue>,
    pub session: BTreeMap<String, Value>,
    pub context: Vec<ContextEvidence>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StatementEvaluation {
    pub id: String,
    pub effect: String,
    pub source: String,
    pub applicability: Applicability,
    pub reasons: Vec<String>,
    pub assignment_paths: Vec<Vec<String>>,
    pub delegation_chains: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AssignmentEvaluation {
    pub id: String,
    pub applicability: Applicability,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SnapshotBinding {
    pub id: String,
    pub digest: String,
    pub retrieval: String,
    pub schema: String,
    pub resolver_version: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PureAuthorizationResult {
    pub schema: String,
    pub request_id: String,
    pub snapshot: SnapshotBinding,
    pub principal: String,
    pub operation: String,
    pub targets: Vec<String>,
    pub decision: String,
    pub reason: String,
    pub effective_roles: Vec<String>,
    pub indeterminate_roles: Vec<String>,
    pub assignments: Vec<AssignmentEvaluation>,
    pub statements: Vec<StatementEvaluation>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct PolicyFiles {
    pub environment: PathBuf,
    pub records: Vec<PathBuf>,
}
