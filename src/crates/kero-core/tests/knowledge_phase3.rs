use kero_core::knowledge::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn rid(c: char) -> SourceRegionId {
    SourceRegionId::new(format!("region:sha256:{}", c.to_string().repeat(64))).unwrap()
}

fn provenance(region: &SourceRegionId, source: &str, revision: char) -> Provenance {
    Provenance {
        source_id: SourceId::new(source).unwrap(),
        revision_id: SourceRevisionId::new(format!("sha256:{}", revision.to_string().repeat(64)))
            .unwrap(),
        region_id: Some(region.clone()),
    }
}

fn explicit(region: &SourceRegionId, source: &str, revision: char) -> Origin {
    Origin::Explicit {
        provenance: vec![provenance(region, source, revision)],
    }
}

fn region_binding(region: &SourceRegionId, source: &str, revision: char) -> SemanticSourceRegion {
    SemanticSourceRegion {
        id: region.clone(),
        source_id: SourceId::new(source).unwrap(),
        revision_id: SourceRevisionId::new(format!("sha256:{}", revision.to_string().repeat(64)))
            .unwrap(),
    }
}

fn claim(
    name: &str,
    subject: &EntityId,
    predicate: &str,
    value: Value,
    polarity: Polarity,
    origin: Origin,
) -> Claim {
    let id = named_record_id(name).unwrap();
    let assertion_revision = assertion_revision(&(subject, predicate, &value, &polarity));
    Claim {
        id,
        assertion_revision,
        subject: subject.clone(),
        predicate: predicate.into(),
        value,
        polarity,
        origin,
    }
}

fn relation(
    id: SemanticRecordId,
    kind: &str,
    from: Endpoint,
    to: Endpoint,
    origin: Origin,
) -> Relation {
    let assertion_revision = assertion_revision(&(kind, &from, &to));
    Relation {
        id,
        assertion_revision,
        kind: kind.into(),
        from,
        to,
        origin,
    }
}

fn realistic_ir() -> SemanticIr {
    let regions = [rid('a'), rid('b'), rid('c')];
    let frog = EntityId::new("sem:animal.frog").unwrap();
    let pond = EntityId::new("sem:place.pond").unwrap();
    let mut ir = SemanticIr::new(KnowledgeSetId::new("field-notes").unwrap());
    ir.source_regions = vec![
        region_binding(&regions[0], "source.guide", '1'),
        region_binding(&regions[1], "source.notes", '2'),
        region_binding(&regions[2], "source.revision", '3'),
    ];
    ir.entities = vec![
        Entity {
            id: frog.clone(),
            kind: Some("biology:species".into()),
            origin: explicit(&regions[0], "source.guide", '1'),
            extensions: BTreeMap::new(),
        },
        Entity {
            id: pond.clone(),
            kind: Some("geography:habitat".into()),
            origin: explicit(&regions[0], "source.guide", '1'),
            extensions: BTreeMap::new(),
        },
    ];
    let green_a = claim(
        "claim.frog-color-guide",
        &frog,
        "biology:color",
        Value::Text("green".into()),
        Polarity::Positive,
        explicit(&regions[0], "source.guide", '1'),
    );
    let green_b = claim(
        "claim.frog-color-notes",
        &frog,
        "biology:color",
        Value::Text("green".into()),
        Polarity::Positive,
        explicit(&regions[1], "source.notes", '2'),
    );
    let not_green = claim(
        "claim.frog-not-green",
        &frog,
        "biology:color",
        Value::Text("green".into()),
        Polarity::Negative,
        explicit(&regions[2], "source.revision", '3'),
    );
    ir.claims = vec![green_a.clone(), green_b, not_green.clone()];
    ir.relations.push(relation(
        named_record_id("relation.frog-contained-by-pond").unwrap(),
        "core:contained-by",
        Endpoint::Entity(frog.clone()),
        Endpoint::Entity(pond),
        explicit(&regions[0], "source.guide", '1'),
    ));
    ir.relations.push(relation(
        named_record_id("relation.new-supersedes-old").unwrap(),
        "core:supersedes",
        Endpoint::Record(not_green.id.clone()),
        Endpoint::Record(green_a.id.clone()),
        explicit(&regions[2], "source.revision", '3'),
    ));
    ir.relations.push(relation(
        named_record_id("relation.frog-seasonal-observation").unwrap(),
        "field-notes:seasonal-observation",
        Endpoint::Entity(frog),
        Endpoint::Record(not_green.id.clone()),
        explicit(&regions[2], "source.revision", '3'),
    ));

    let green_a_id = green_a.id.clone();
    let inputs = DerivationInputs::Unordered(vec![green_a.id, not_green.id.clone()]);
    let derivation_id = derive_derivation_id(
        &inputs,
        "core:explanation",
        "kero-core",
        "0.1.0",
        &BTreeMap::new(),
    );
    let output = derived_record_id(&derivation_id, "relation", "color-conflict");
    ir.relations.push(relation(
        output.clone(),
        "core:explains",
        Endpoint::Record(green_a_id),
        Endpoint::Record(not_green.id),
        Origin::Derived {
            derivation_id: derivation_id.clone(),
        },
    ));
    ir.derivations.push(Derivation {
        id: derivation_id,
        inputs,
        kind: "core:explanation".into(),
        implementation: "kero-core".into(),
        version: "0.1.0".into(),
        parameters: BTreeMap::new(),
        outputs: vec![output],
    });
    ir
}

#[test]
fn knowledge_semantic_preserves_agreement_contradiction_and_unknown_relations() {
    let ir = realistic_ir();
    ir.validate().unwrap();
    assert_eq!(ir.claims.len(), 3);
    assert!(
        ir.claims
            .iter()
            .any(|claim| claim.polarity == Polarity::Negative)
    );
    assert!(
        ir.relations
            .iter()
            .any(|relation| relation.kind == "field-notes:seasonal-observation")
    );
    assert!(matches!(
        ir.relations.last().unwrap().origin,
        Origin::Derived { .. }
    ));
}

#[test]
fn knowledge_semantic_named_identity_survives_region_change_but_revision_does_not() {
    let frog = EntityId::new("sem:animal.frog").unwrap();
    let first = claim(
        "claim.stable",
        &frog,
        "biology:color",
        Value::Text("green".into()),
        Polarity::Positive,
        explicit(&rid('a'), "source.a", '1'),
    );
    let moved = claim(
        "claim.stable",
        &frog,
        "biology:color",
        Value::Text("blue".into()),
        Polarity::Positive,
        explicit(&rid('b'), "source.a", '2'),
    );
    assert_eq!(first.id, moved.id);
    assert_ne!(first.assertion_revision, moved.assertion_revision);
}

#[test]
fn knowledge_canonical_distinguishes_meaningful_list_order_only() {
    let mut first = realistic_ir();
    let mut reordered = first.clone();
    reordered.entities.reverse();
    reordered.claims.reverse();
    reordered.relations.reverse();
    reordered.source_regions.reverse();
    assert_eq!(
        canonicalize(&first).unwrap(),
        canonicalize(&reordered).unwrap()
    );
    first.claims[0].value = Value::List(vec![Value::Text("a".into()), Value::Text("b".into())]);
    first.claims[0].assertion_revision = assertion_revision(&(
        &first.claims[0].subject,
        &first.claims[0].predicate,
        &first.claims[0].value,
        &first.claims[0].polarity,
    ));
    let mut second = first.clone();
    if let Value::List(values) = &mut second.claims[0].value {
        values.reverse();
    }
    second.claims[0].assertion_revision = assertion_revision(&(
        &second.claims[0].subject,
        &second.claims[0].predicate,
        &second.claims[0].value,
        &second.claims[0].polarity,
    ));
    assert_ne!(
        encode(&canonicalize(&first).unwrap(), false).unwrap(),
        encode(&canonicalize(&second).unwrap(), false).unwrap()
    );
}

#[test]
fn knowledge_encoding_is_deterministic_and_symbol_table_is_semantically_transparent() {
    let canonical = canonicalize(&realistic_ir()).unwrap();
    let plain = encode(&canonical, false).unwrap();
    let symbols = encode(&canonical, true).unwrap();
    assert_eq!(decode(&plain).unwrap(), decode(&symbols).unwrap());
    assert_eq!(encode(&decode(&plain).unwrap(), false).unwrap(), plain);
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    std::fs::write(first.path().join("knowledge.json"), &plain).unwrap();
    std::fs::write(
        second.path().join("knowledge.json"),
        encode(&canonical, false).unwrap(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(first.path().join("knowledge.json")).unwrap(),
        std::fs::read(second.path().join("knowledge.json")).unwrap()
    );
}

#[test]
fn knowledge_semantic_reports_stable_corruption_codes() {
    let mut ir = realistic_ir();
    ir.claims[0].subject = EntityId::new("sem:missing").unwrap();
    assert_eq!(
        ir.validate().unwrap_err().code(),
        "semantic.entity-dangling"
    );
    let canonical = canonicalize(&realistic_ir()).unwrap();
    let mut bytes = encode(&canonical, false).unwrap();
    bytes.pop();
    assert_eq!(decode(&bytes).unwrap_err().code(), "encoding.noncanonical");
}

#[test]
fn knowledge_encoding_rejects_corrupt_ids_tables_provenance_and_ordering() {
    let mut corrupt_id = realistic_ir();
    corrupt_id.claims[0].assertion_revision = format!("sha256:{}", "0".repeat(64));
    assert_eq!(
        corrupt_id.validate().unwrap_err().code(),
        "semantic.id-corrupt"
    );

    let mut duplicate = realistic_ir();
    duplicate.claims.push(duplicate.claims[0].clone());
    assert_eq!(
        duplicate.validate().unwrap_err().code(),
        "semantic.id-duplicate"
    );

    let mut missing_provenance = realistic_ir();
    missing_provenance.entities[0].origin = Origin::Explicit { provenance: vec![] };
    assert_eq!(
        missing_provenance.validate().unwrap_err().code(),
        "semantic.provenance-missing"
    );

    let mut mismatched_binding = realistic_ir();
    mismatched_binding.source_regions[0].source_id = SourceId::new("source.wrong").unwrap();
    assert_eq!(
        mismatched_binding.validate().unwrap_err().code(),
        "semantic.provenance-missing"
    );

    let canonical = canonicalize(&realistic_ir()).unwrap();
    let symbol_bytes = encode(&canonical, true).unwrap();
    let mut symbol_json: serde_json::Value = serde_json::from_slice(&symbol_bytes).unwrap();
    symbol_json["symbols"] = serde_json::json!(["wrong:symbol"]);
    let mut invalid_symbols = serde_json::to_vec(&symbol_json).unwrap();
    invalid_symbols.push(b'\n');
    assert_eq!(
        decode(&invalid_symbols).unwrap_err().code(),
        "encoding.symbol-table-invalid"
    );

    let plain = encode(&canonical, false).unwrap();
    let mut unordered: serde_json::Value = serde_json::from_slice(&plain).unwrap();
    unordered["knowledge"]["ir"]["claims"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let mut unordered_bytes = serde_json::to_vec(&unordered).unwrap();
    unordered_bytes.push(b'\n');
    assert_eq!(
        decode(&unordered_bytes).unwrap_err().code(),
        "encoding.noncanonical"
    );
}

#[test]
fn knowledge_semantic_rejects_required_unresolved_reference() {
    let mut ir = realistic_ir();
    ir.references.push(Reference {
        id: named_record_id("reference.required").unwrap(),
        surface: "unknown".into(),
        required: true,
        state: ReferenceState::Unresolved { candidates: vec![] },
        origin: explicit(&rid('a'), "source.guide", '1'),
    });
    assert_eq!(
        ir.validate().unwrap_err().code(),
        "semantic.reference-state-invalid"
    );
}

#[test]
fn knowledge_semantic_rejects_noncanonical_decimal() {
    let mut ir = realistic_ir();
    ir.claims[0].value = Value::Decimal("01.20".into());
    ir.claims[0].assertion_revision = assertion_revision(&(
        &ir.claims[0].subject,
        &ir.claims[0].predicate,
        &ir.claims[0].value,
        &ir.claims[0].polarity,
    ));
    assert_eq!(
        ir.validate().unwrap_err().code(),
        "semantic.decimal-noncanonical"
    );
}

#[test]
fn canonical_fixture_hash_is_stable() {
    let bytes = encode(&canonicalize(&realistic_ir()).unwrap(), false).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    assert_eq!(
        digest,
        include_str!("fixtures/knowledge/phase3-canonical.sha256").trim()
    );
}
