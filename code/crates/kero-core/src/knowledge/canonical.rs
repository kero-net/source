//! Semantic normalization, deliberately separate from byte encoding.

use serde::{Deserialize, Serialize};

use super::{DerivationInputs, SemanticError, SemanticIr};

pub const CANONICAL_VERSION: &str = "kero/canonicalization/v1alpha1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalKnowledge {
    pub version: String,
    pub ir: SemanticIr,
}

pub fn canonicalize(ir: &SemanticIr) -> Result<CanonicalKnowledge, SemanticError> {
    ir.validate()?;
    let mut ir = ir.clone();
    ir.source_regions.sort_by(|a, b| a.id.cmp(&b.id));
    ir.entities.sort_by(|a, b| a.id.cmp(&b.id));
    ir.references.sort_by(|a, b| a.id.cmp(&b.id));
    ir.claims.sort_by(|a, b| a.id.cmp(&b.id));
    ir.relations.sort_by(|a, b| a.id.cmp(&b.id));
    ir.derivations.sort_by(|a, b| a.id.cmp(&b.id));
    for entity in &mut ir.entities {
        canonical_origin(&mut entity.origin);
    }
    for reference in &mut ir.references {
        canonical_origin(&mut reference.origin);
        match &mut reference.state {
            super::ReferenceState::Resolved { evidence, .. } => sort_json(evidence),
            super::ReferenceState::Ambiguous { candidates } => {
                for candidate in candidates.iter_mut() {
                    sort_json(&mut candidate.evidence);
                }
                candidates.sort_by(|a, b| a.target.cmp(&b.target));
            }
            super::ReferenceState::Unresolved { .. } => {}
        }
    }
    for claim in &mut ir.claims {
        canonical_origin(&mut claim.origin);
    }
    for relation in &mut ir.relations {
        canonical_origin(&mut relation.origin);
    }
    for derivation in &mut ir.derivations {
        if let DerivationInputs::Unordered(ids) = &mut derivation.inputs {
            ids.sort();
        }
        derivation.outputs.sort();
    }
    Ok(CanonicalKnowledge {
        version: CANONICAL_VERSION.into(),
        ir,
    })
}

fn canonical_origin(origin: &mut super::Origin) {
    if let super::Origin::Explicit { provenance } = origin {
        sort_json(provenance);
    }
}

fn sort_json<T: Serialize>(values: &mut [T]) {
    values.sort_by_cached_key(|value| {
        serde_json::to_vec(value).expect("serializable semantic value")
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{
        KnowledgeSetId, SemanticSourceRegion, SourceId, SourceRegionId, SourceRevisionId,
    };

    #[test]
    fn knowledge_canonical_removes_meaningless_input_order() {
        let mut a = SemanticIr::new(KnowledgeSetId::new("demo").unwrap());
        a.source_regions = ['b', 'a']
            .into_iter()
            .map(|c| SemanticSourceRegion {
                id: SourceRegionId::new(format!("region:sha256:{}", c.to_string().repeat(64)))
                    .unwrap(),
                source_id: SourceId::new(format!("source.{c}")).unwrap(),
                revision_id: SourceRevisionId::new(format!("sha256:{}", c.to_string().repeat(64)))
                    .unwrap(),
            })
            .collect();
        let mut b = a.clone();
        b.source_regions.reverse();
        assert_eq!(canonicalize(&a).unwrap(), canonicalize(&b).unwrap());
    }
}
