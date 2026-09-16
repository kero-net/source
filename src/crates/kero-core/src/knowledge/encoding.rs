//! Deterministic and inspectable canonical JSON envelope.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

use super::{CanonicalKnowledge, SemanticError, canonicalize};

pub const ENCODING_SCHEMA: &str = "kero/canonical-json/v1alpha1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema: String,
    canonicalization: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbols: Option<Vec<String>>,
    knowledge: CanonicalKnowledge,
}

pub fn encode(
    knowledge: &CanonicalKnowledge,
    include_symbol_table: bool,
) -> Result<Vec<u8>, EncodingError> {
    let expected = canonicalize(&knowledge.ir)?;
    if &expected != knowledge {
        return Err(EncodingError::Noncanonical);
    }
    let symbols = include_symbol_table.then(|| collect_symbols(knowledge));
    let envelope = Envelope {
        schema: ENCODING_SCHEMA.into(),
        canonicalization: super::CANONICAL_VERSION.into(),
        symbols,
        knowledge: knowledge.clone(),
    };
    let mut bytes = serde_json::to_vec(&envelope)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn decode(bytes: &[u8]) -> Result<CanonicalKnowledge, EncodingError> {
    if !bytes.ends_with(b"\n") {
        return Err(EncodingError::Noncanonical);
    }
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.schema != ENCODING_SCHEMA {
        return Err(EncodingError::Schema(envelope.schema));
    }
    if envelope.canonicalization != super::CANONICAL_VERSION {
        return Err(EncodingError::CanonicalVersion(envelope.canonicalization));
    }
    if let Some(symbols) = &envelope.symbols {
        if symbols != &collect_symbols(&envelope.knowledge) {
            return Err(EncodingError::SymbolTable);
        }
    }
    let expected = canonicalize(&envelope.knowledge.ir)?;
    if expected != envelope.knowledge {
        return Err(EncodingError::Noncanonical);
    }
    if encode(&envelope.knowledge, envelope.symbols.is_some())? != bytes {
        return Err(EncodingError::Noncanonical);
    }
    Ok(envelope.knowledge)
}

fn collect_symbols(knowledge: &CanonicalKnowledge) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    for entity in &knowledge.ir.entities {
        if let Some(kind) = &entity.kind {
            symbols.insert(kind.clone());
        }
    }
    for claim in &knowledge.ir.claims {
        symbols.insert(claim.predicate.clone());
    }
    for relation in &knowledge.ir.relations {
        symbols.insert(relation.kind.clone());
    }
    for derivation in &knowledge.ir.derivations {
        symbols.insert(derivation.kind.clone());
        symbols.insert(derivation.implementation.clone());
        symbols.insert(derivation.version.clone());
    }
    symbols.into_iter().collect()
}

#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("encoding.json-invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("encoding.schema-unsupported: {0}")]
    Schema(String),
    #[error("encoding.canonicalization-unsupported: {0}")]
    CanonicalVersion(String),
    #[error("encoding.symbol-table-invalid")]
    SymbolTable,
    #[error("encoding.noncanonical")]
    Noncanonical,
    #[error(transparent)]
    Semantic(#[from] SemanticError),
}

impl EncodingError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "encoding.json-invalid",
            Self::Schema(_) => "encoding.schema-unsupported",
            Self::CanonicalVersion(_) => "encoding.canonicalization-unsupported",
            Self::SymbolTable => "encoding.symbol-table-invalid",
            Self::Noncanonical => "encoding.noncanonical",
            Self::Semantic(error) => error.code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{KnowledgeSetId, SemanticIr};

    #[test]
    fn knowledge_encoding_round_trips_with_and_without_symbols() {
        let ir = SemanticIr::new(KnowledgeSetId::new("demo").unwrap());
        let canonical = canonicalize(&ir).unwrap();
        for symbols in [false, true] {
            let bytes = encode(&canonical, symbols).unwrap();
            assert_eq!(decode(&bytes).unwrap(), canonical);
            assert_eq!(encode(&decode(&bytes).unwrap(), symbols).unwrap(), bytes);
        }
    }

    #[test]
    fn knowledge_encoding_rejects_noncanonical_bytes() {
        let ir = SemanticIr::new(KnowledgeSetId::new("demo").unwrap());
        let canonical = canonicalize(&ir).unwrap();
        let mut bytes = encode(&canonical, false).unwrap();
        bytes.pop();
        assert!(matches!(decode(&bytes), Err(EncodingError::Noncanonical)));
    }
}
