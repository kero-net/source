//! Versioned semantic intermediate representation and boundary validation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use super::{
    DerivationId, EntityId, KnowledgeSetId, Provenance, SemanticRecordId, SourceId, SourceRegionId,
    SourceRevisionId,
};

pub const SEMANTIC_IR_VERSION: &str = "kero/semantic-ir/v1alpha1";
pub const EXPLICIT_ID_ALGORITHM: &str = "semantic-explicit/sha256-v1";
pub const ASSERTION_REVISION_ALGORITHM: &str = "semantic-assertion/sha256-v1";
pub const DERIVATION_ID_ALGORITHM: &str = "semantic-derivation/sha256-v1";
pub const DERIVED_RECORD_ID_ALGORITHM: &str = "semantic-derived-record/sha256-v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticIr {
    pub version: String,
    pub knowledge_set_id: KnowledgeSetId,
    #[serde(default)]
    pub source_regions: Vec<SemanticSourceRegion>,
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub references: Vec<Reference>,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub relations: Vec<Relation>,
    #[serde(default)]
    pub derivations: Vec<Derivation>,
}

impl SemanticIr {
    pub fn new(knowledge_set_id: KnowledgeSetId) -> Self {
        Self {
            version: SEMANTIC_IR_VERSION.into(),
            knowledge_set_id,
            source_regions: vec![],
            entities: vec![],
            references: vec![],
            claims: vec![],
            relations: vec![],
            derivations: vec![],
        }
    }

    pub fn validate(&self) -> Result<(), SemanticError> {
        if self.version != SEMANTIC_IR_VERSION {
            return Err(SemanticError::Version(self.version.clone()));
        }
        let regions: BTreeMap<_, _> = self
            .source_regions
            .iter()
            .map(|region| (&region.id, region))
            .collect();
        if regions.len() != self.source_regions.len() {
            return Err(SemanticError::DuplicateId("source-region"));
        }
        let entities: BTreeSet<_> = self.entities.iter().map(|v| &v.id).collect();
        if entities.len() != self.entities.len() {
            return Err(SemanticError::DuplicateId("entity"));
        }
        let mut records = BTreeSet::new();
        for id in self
            .references
            .iter()
            .map(|v| &v.id)
            .chain(self.claims.iter().map(|v| &v.id))
            .chain(self.relations.iter().map(|v| &v.id))
        {
            if !records.insert(id) {
                return Err(SemanticError::DuplicateId("semantic-record"));
            }
        }
        let derivations: BTreeSet<_> = self.derivations.iter().map(|v| &v.id).collect();
        if derivations.len() != self.derivations.len() {
            return Err(SemanticError::DuplicateId("derivation"));
        }
        for entity in &self.entities {
            if let Some(kind) = &entity.kind {
                validate_symbol(kind)?;
            }
            validate_origin(&entity.origin, &regions, &derivations)?;
        }
        for reference in &self.references {
            validate_origin(&reference.origin, &regions, &derivations)?;
            match &reference.state {
                ReferenceState::Unresolved { candidates }
                    if reference.required || !candidates.is_empty() =>
                {
                    return Err(SemanticError::ReferenceState(reference.id.clone()));
                }
                ReferenceState::Resolved { target, evidence } => {
                    if !entities.contains(target) || evidence.is_empty() {
                        return Err(SemanticError::ReferenceState(reference.id.clone()));
                    }
                    validate_provenance(evidence, &regions)?;
                }
                ReferenceState::Ambiguous { candidates } => {
                    if candidates.len() < 2 {
                        return Err(SemanticError::ReferenceState(reference.id.clone()));
                    }
                    for candidate in candidates {
                        if !entities.contains(&candidate.target) || candidate.evidence.is_empty() {
                            return Err(SemanticError::ReferenceState(reference.id.clone()));
                        }
                        validate_provenance(&candidate.evidence, &regions)?;
                    }
                }
                _ => {}
            }
        }
        for claim in &self.claims {
            if !entities.contains(&claim.subject) {
                return Err(SemanticError::DanglingEntity(claim.subject.clone()));
            }
            validate_symbol(&claim.predicate)?;
            validate_origin(&claim.origin, &regions, &derivations)?;
            validate_value(&claim.value, &entities)?;
            verify_assertion_revision(
                &claim.id,
                &claim.assertion_revision,
                &(
                    &claim.subject,
                    &claim.predicate,
                    &claim.value,
                    &claim.polarity,
                ),
            )?;
        }
        for relation in &self.relations {
            validate_symbol(&relation.kind)?;
            validate_endpoint(&relation.from, &entities, &records)?;
            validate_endpoint(&relation.to, &entities, &records)?;
            validate_origin(&relation.origin, &regions, &derivations)?;
            verify_assertion_revision(
                &relation.id,
                &relation.assertion_revision,
                &(&relation.kind, &relation.from, &relation.to),
            )?;
        }
        for derivation in &self.derivations {
            validate_symbol(&derivation.kind)?;
            if derivation.inputs.values().is_empty()
                || derivation.implementation.trim().is_empty()
                || derivation.version.trim().is_empty()
            {
                return Err(SemanticError::DerivationMetadata(derivation.id.clone()));
            }
            for input in derivation.inputs.values() {
                if !records.contains(input) {
                    return Err(SemanticError::DanglingRecord(input.clone()));
                }
            }
            for output in &derivation.outputs {
                if !records.contains(output) {
                    return Err(SemanticError::DanglingRecord(output.clone()));
                }
            }
            if derivation.id
                != derive_derivation_id(
                    &derivation.inputs,
                    &derivation.kind,
                    &derivation.implementation,
                    &derivation.version,
                    &derivation.parameters,
                )
            {
                return Err(SemanticError::CorruptId(derivation.id.to_string()));
            }
            for output in &derivation.outputs {
                let origin = self
                    .references
                    .iter()
                    .find(|record| &record.id == output)
                    .map(|record| &record.origin)
                    .or_else(|| {
                        self.claims
                            .iter()
                            .find(|record| &record.id == output)
                            .map(|record| &record.origin)
                    })
                    .or_else(|| {
                        self.relations
                            .iter()
                            .find(|record| &record.id == output)
                            .map(|record| &record.origin)
                    });
                if !matches!(origin, Some(Origin::Derived { derivation_id }) if derivation_id == &derivation.id)
                {
                    return Err(SemanticError::DerivationOutput(output.clone()));
                }
            }
        }
        for (id, origin) in self
            .references
            .iter()
            .map(|v| (&v.id, &v.origin))
            .chain(self.claims.iter().map(|v| (&v.id, &v.origin)))
            .chain(self.relations.iter().map(|v| (&v.id, &v.origin)))
        {
            if let Origin::Derived { derivation_id } = origin {
                let owner = self.derivations.iter().find(|d| &d.id == derivation_id);
                if !owner.is_some_and(|d| d.outputs.contains(id)) {
                    return Err(SemanticError::DerivationOutput(id.clone()));
                }
            }
        }
        validate_derivation_cycles(self)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: EntityId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticSourceRegion {
    pub id: SourceRegionId,
    pub source_id: SourceId,
    pub revision_id: SourceRevisionId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "origin")]
pub enum Origin {
    Explicit { provenance: Vec<Provenance> },
    Derived { derivation_id: DerivationId },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "value")]
pub enum Value {
    Null,
    Boolean(bool),
    Integer(i64),
    Decimal(String),
    Text(String),
    Entity(EntityId),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Opaque(OpaqueValue),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OpaqueValue {
    pub kind: String,
    pub version: String,
    pub data: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Polarity {
    Positive,
    Negative,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub id: SemanticRecordId,
    pub assertion_revision: String,
    pub subject: EntityId,
    pub predicate: String,
    pub value: Value,
    pub polarity: Polarity,
    pub origin: Origin,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "id")]
pub enum Endpoint {
    Entity(EntityId),
    Record(SemanticRecordId),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub id: SemanticRecordId,
    pub assertion_revision: String,
    pub kind: String,
    pub from: Endpoint,
    pub to: Endpoint,
    pub origin: Origin,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub id: SemanticRecordId,
    pub surface: String,
    pub required: bool,
    pub state: ReferenceState,
    pub origin: Origin,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum ReferenceState {
    Unresolved {
        #[serde(default)]
        candidates: Vec<ReferenceCandidate>,
    },
    Resolved {
        target: EntityId,
        evidence: Vec<Provenance>,
    },
    Ambiguous {
        candidates: Vec<ReferenceCandidate>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceCandidate {
    pub target: EntityId,
    pub evidence: Vec<Provenance>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "order", content = "ids")]
pub enum DerivationInputs {
    Ordered(Vec<SemanticRecordId>),
    Unordered(Vec<SemanticRecordId>),
}
impl DerivationInputs {
    pub fn values(&self) -> &[SemanticRecordId] {
        match self {
            Self::Ordered(v) | Self::Unordered(v) => v,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Derivation {
    pub id: DerivationId,
    pub inputs: DerivationInputs,
    pub kind: String,
    pub implementation: String,
    pub version: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Value>,
    pub outputs: Vec<SemanticRecordId>,
}

pub fn explicit_record_id(
    region: &SourceRegionId,
    kind: &str,
    discriminator: &str,
) -> SemanticRecordId {
    hash_record(EXPLICIT_ID_ALGORITHM, &(region, kind, discriminator))
}
pub fn named_record_id(name: &str) -> Result<SemanticRecordId, super::IdError> {
    SemanticRecordId::new(format!("record:{name}"))
}
pub fn derived_record_id(
    derivation: &DerivationId,
    record_kind: &str,
    discriminator: &str,
) -> SemanticRecordId {
    hash_record(
        DERIVED_RECORD_ID_ALGORITHM,
        &(derivation, record_kind, discriminator),
    )
}
pub fn assertion_revision<T: Serialize>(value: &T) -> String {
    format!("sha256:{}", hash_json(ASSERTION_REVISION_ALGORITHM, value))
}
pub fn derive_derivation_id(
    inputs: &DerivationInputs,
    kind: &str,
    implementation: &str,
    version: &str,
    parameters: &BTreeMap<String, Value>,
) -> DerivationId {
    let normalized_inputs = match inputs {
        DerivationInputs::Ordered(ids) => DerivationInputs::Ordered(ids.clone()),
        DerivationInputs::Unordered(ids) => {
            let mut ids = ids.clone();
            ids.sort();
            DerivationInputs::Unordered(ids)
        }
    };
    DerivationId::new(format!(
        "derivation:sha256:{}",
        hash_json(
            DERIVATION_ID_ALGORITHM,
            &(normalized_inputs, kind, implementation, version, parameters)
        )
    ))
    .expect("generated ID")
}

fn hash_record<T: Serialize>(domain: &str, value: &T) -> SemanticRecordId {
    SemanticRecordId::new(format!("record:sha256:{}", hash_json(domain, value)))
        .expect("generated ID")
}
fn hash_json<T: Serialize>(domain: &str, value: &T) -> String {
    let mut h = Sha256::new();
    h.update((domain.len() as u64).to_be_bytes());
    h.update(domain);
    let bytes = serde_json::to_vec(value).expect("semantic identity is serializable");
    h.update((bytes.len() as u64).to_be_bytes());
    h.update(bytes);
    hex::encode(h.finalize())
}
fn validate_symbol(value: &str) -> Result<(), SemanticError> {
    let mut parts = value.split(':');
    let namespace = parts.next().unwrap_or("");
    let name = parts.next().unwrap_or("");
    if parts.next().is_some() || !logical_symbol(namespace) || !logical_symbol(name) {
        Err(SemanticError::Symbol(value.into()))
    } else {
        Ok(())
    }
}
fn logical_symbol(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 128
        && v.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase()
                || b.is_ascii_digit()
                || (i > 0 && (b == b'.' || b == b'-' || b == b'_'))
        })
}
fn validate_origin(
    origin: &Origin,
    regions: &BTreeMap<&SourceRegionId, &SemanticSourceRegion>,
    derivations: &BTreeSet<&DerivationId>,
) -> Result<(), SemanticError> {
    match origin {
        Origin::Explicit { provenance } => validate_provenance(provenance, regions),
        Origin::Derived { derivation_id } if derivations.contains(derivation_id) => Ok(()),
        Origin::Derived { derivation_id } => {
            Err(SemanticError::DanglingDerivation(derivation_id.clone()))
        }
    }
}
fn validate_provenance(
    values: &[Provenance],
    regions: &BTreeMap<&SourceRegionId, &SemanticSourceRegion>,
) -> Result<(), SemanticError> {
    if values.is_empty() {
        return Err(SemanticError::MissingProvenance);
    }
    for p in values {
        match &p.region_id {
            Some(id)
                if regions.get(id).is_some_and(|region| {
                    region.source_id == p.source_id && region.revision_id == p.revision_id
                }) => {}
            _ => return Err(SemanticError::MissingProvenance),
        }
    }
    Ok(())
}
fn validate_value(value: &Value, entities: &BTreeSet<&EntityId>) -> Result<(), SemanticError> {
    match value {
        Value::Decimal(v) if !normalized_decimal(v) => Err(SemanticError::Decimal(v.clone())),
        Value::Entity(id) if !entities.contains(id) => {
            Err(SemanticError::DanglingEntity(id.clone()))
        }
        Value::List(v) => v.iter().try_for_each(|v| validate_value(v, entities)),
        Value::Map(v) => v.values().try_for_each(|v| validate_value(v, entities)),
        Value::Opaque(v) => {
            validate_symbol(&v.kind).map_err(|_| SemanticError::Opaque)?;
            if v.version.trim().is_empty() {
                Err(SemanticError::Opaque)
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}
fn normalized_decimal(v: &str) -> bool {
    if v == "0" {
        return true;
    }
    let v = v.strip_prefix('-').unwrap_or(v);
    if v.is_empty()
        || (v.starts_with('0') && v.len() > 1 && !v.starts_with("0."))
        || (v.contains('.') && v.ends_with('0'))
        || v.ends_with('.')
    {
        return false;
    }
    let mut dots = 0;
    v.bytes().all(|b| {
        if b == b'.' {
            dots += 1;
            dots == 1
        } else {
            b.is_ascii_digit()
        }
    })
}
fn validate_endpoint(
    v: &Endpoint,
    entities: &BTreeSet<&EntityId>,
    records: &BTreeSet<&SemanticRecordId>,
) -> Result<(), SemanticError> {
    match v {
        Endpoint::Entity(id) if !entities.contains(id) => {
            Err(SemanticError::DanglingEntity(id.clone()))
        }
        Endpoint::Record(id) if !records.contains(id) => {
            Err(SemanticError::DanglingRecord(id.clone()))
        }
        _ => Ok(()),
    }
}
fn verify_assertion_revision<T: Serialize>(
    id: &SemanticRecordId,
    actual: &str,
    value: &T,
) -> Result<(), SemanticError> {
    if actual == assertion_revision(value) {
        Ok(())
    } else {
        Err(SemanticError::CorruptId(id.to_string()))
    }
}
fn validate_derivation_cycles(ir: &SemanticIr) -> Result<(), SemanticError> {
    let owners: BTreeMap<_, _> = ir
        .derivations
        .iter()
        .flat_map(|d| d.outputs.iter().map(move |o| (o, &d.id)))
        .collect();
    let graph: BTreeMap<_, _> = ir
        .derivations
        .iter()
        .map(|d| {
            (
                &d.id,
                d.inputs
                    .values()
                    .iter()
                    .filter_map(|i| owners.get(i).copied())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    fn visit<'a>(
        n: &'a DerivationId,
        g: &BTreeMap<&'a DerivationId, Vec<&'a DerivationId>>,
        active: &mut BTreeSet<&'a DerivationId>,
        done: &mut BTreeSet<&'a DerivationId>,
    ) -> bool {
        if done.contains(n) {
            return false;
        }
        if !active.insert(n) {
            return true;
        }
        if g.get(n)
            .is_some_and(|v| v.iter().any(|x| visit(x, g, active, done)))
        {
            return true;
        }
        active.remove(n);
        done.insert(n);
        false
    }
    let mut active = BTreeSet::new();
    let mut done = BTreeSet::new();
    if graph
        .keys()
        .any(|n| visit(n, &graph, &mut active, &mut done))
    {
        Err(SemanticError::DerivationCycle)
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum SemanticError {
    #[error("semantic.version-unsupported: {0}")]
    Version(String),
    #[error("semantic.id-duplicate: {0}")]
    DuplicateId(&'static str),
    #[error("semantic.symbol-invalid: {0}")]
    Symbol(String),
    #[error("semantic.provenance-missing")]
    MissingProvenance,
    #[error("semantic.entity-dangling: {0}")]
    DanglingEntity(EntityId),
    #[error("semantic.record-dangling: {0}")]
    DanglingRecord(SemanticRecordId),
    #[error("semantic.derivation-dangling: {0}")]
    DanglingDerivation(DerivationId),
    #[error("semantic.reference-state-invalid: {0}")]
    ReferenceState(SemanticRecordId),
    #[error("semantic.decimal-noncanonical: {0}")]
    Decimal(String),
    #[error("semantic.opaque-invalid")]
    Opaque,
    #[error("semantic.derivation-metadata-invalid: {0}")]
    DerivationMetadata(DerivationId),
    #[error("semantic.derivation-cycle")]
    DerivationCycle,
    #[error("semantic.derivation-output-invalid: {0}")]
    DerivationOutput(SemanticRecordId),
    #[error("semantic.id-corrupt: {0}")]
    CorruptId(String),
}

impl SemanticError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Version(_) => "semantic.version-unsupported",
            Self::DuplicateId(_) => "semantic.id-duplicate",
            Self::Symbol(_) => "semantic.symbol-invalid",
            Self::MissingProvenance => "semantic.provenance-missing",
            Self::DanglingEntity(_) => "semantic.entity-dangling",
            Self::DanglingRecord(_) => "semantic.record-dangling",
            Self::DanglingDerivation(_) => "semantic.derivation-dangling",
            Self::ReferenceState(_) => "semantic.reference-state-invalid",
            Self::Decimal(_) => "semantic.decimal-noncanonical",
            Self::Opaque => "semantic.opaque-invalid",
            Self::DerivationMetadata(_) => "semantic.derivation-metadata-invalid",
            Self::DerivationCycle => "semantic.derivation-cycle",
            Self::DerivationOutput(_) => "semantic.derivation-output-invalid",
            Self::CorruptId(_) => "semantic.id-corrupt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{SourceId, SourceRevisionId};

    fn region() -> SourceRegionId {
        SourceRegionId::new(format!("region:sha256:{}", "a".repeat(64))).unwrap()
    }

    #[test]
    fn knowledge_semantic_explicit_and_derived_ids_have_distinct_domains() {
        let region = region();
        let explicit = explicit_record_id(&region, "claim", "first");
        let inputs = DerivationInputs::Ordered(vec![explicit.clone()]);
        let derivation =
            derive_derivation_id(&inputs, "core:copy", "kero-core", "1", &BTreeMap::new());
        let derived = derived_record_id(&derivation, "claim", "first");
        assert!(explicit.as_str().starts_with("record:sha256:"));
        assert!(derivation.as_str().starts_with("derivation:sha256:"));
        assert_ne!(explicit, derived);
    }

    #[test]
    fn knowledge_semantic_explicit_origin_requires_registered_region() {
        let origin = Origin::Explicit {
            provenance: vec![Provenance {
                source_id: SourceId::new("source.notes").unwrap(),
                revision_id: SourceRevisionId::new(format!("sha256:{}", "b".repeat(64))).unwrap(),
                region_id: Some(region()),
            }],
        };
        let ir = SemanticIr {
            version: SEMANTIC_IR_VERSION.into(),
            knowledge_set_id: KnowledgeSetId::new("demo").unwrap(),
            source_regions: vec![],
            entities: vec![Entity {
                id: EntityId::new("sem:subject").unwrap(),
                kind: None,
                origin,
                extensions: BTreeMap::new(),
            }],
            references: vec![],
            claims: vec![],
            relations: vec![],
            derivations: vec![],
        };
        assert_eq!(
            ir.validate().unwrap_err().code(),
            "semantic.provenance-missing"
        );
    }
}
