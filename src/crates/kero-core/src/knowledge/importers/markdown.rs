//! Constrained Markdown importer.
//!
//! Semantics are declared in single-line HTML comments:
//! `<!-- kero:entity id="sem:name" kind="domain:type" -->`,
//! `<!-- kero:claim id="name" subject="sem:name" predicate="domain:key" value="text" -->`,
//! and `<!-- kero:relation id="name" kind="domain:kind" from="sem:a" to="sem:b" -->`.
//! Claims accept `polarity="negative"`; all other polarity is positive. Headings,
//! prose, ordinary links, and citation syntax are preserved as regions but emit no
//! semantics. This is intentionally not natural-language extraction.

use std::collections::BTreeMap;

use super::super::*;
use super::Importer;

#[derive(Clone, Debug)]
pub struct MarkdownDocument {
    pub revision: SourceRevision,
    pub text: String,
    pub nodes: Vec<Node>,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub start: usize,
    pub end: usize,
    pub kind: String,
    pub attrs: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct MarkdownImporter;

impl Importer for MarkdownImporter {
    type Acquired = Vec<u8>;
    type Syntax = MarkdownDocument;

    fn acquire(&self, source: &Source, bytes: Vec<u8>) -> Result<Vec<u8>, Diagnostic> {
        if source.media_type != "text/markdown" && source.media_type != "text/x-markdown" {
            return Err(Diagnostic::error(
                "importer.markdown.media-type",
                format!("unsupported media type {}", source.media_type),
            )
            .with_source(source.id.clone()));
        }
        Ok(bytes)
    }

    fn parse(
        &self,
        revision: &SourceRevision,
        input: Vec<u8>,
    ) -> Result<MarkdownDocument, Vec<Diagnostic>> {
        let text = String::from_utf8(input).map_err(|_| {
            vec![
                Diagnostic::error("importer.markdown.utf8", "Markdown must be UTF-8")
                    .with_source(revision.source_id.clone()),
            ]
        })?;
        let mut nodes = Vec::new();
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            if let Some(comment) = line
                .trim()
                .strip_prefix("<!-- kero:")
                .and_then(|s| s.strip_suffix("-->"))
            {
                let comment = comment.trim();
                let (kind, rest) = comment
                    .split_once(char::is_whitespace)
                    .unwrap_or((comment, ""));
                let attrs = parse_attrs(rest).map_err(|message| {
                    vec![
                        Diagnostic::error("importer.markdown.annotation-malformed", message)
                            .with_source(revision.source_id.clone()),
                    ]
                })?;
                if !matches!(kind, "entity" | "claim" | "relation") {
                    return Err(vec![
                        Diagnostic::error(
                            "importer.markdown.annotation-unknown",
                            format!("unknown annotation {kind}"),
                        )
                        .with_source(revision.source_id.clone()),
                    ]);
                }
                let allowed: &[&str] = match kind {
                    "entity" => &["id", "kind"],
                    "claim" => &["id", "subject", "predicate", "value", "polarity"],
                    "relation" => &["id", "kind", "from", "to"],
                    _ => unreachable!(),
                };
                if let Some(key) = attrs.keys().find(|key| !allowed.contains(&key.as_str())) {
                    return Err(vec![
                        Diagnostic::error(
                            "importer.markdown.attribute-unknown",
                            format!("unknown {kind} attribute {key}"),
                        )
                        .with_source(revision.source_id.clone()),
                    ]);
                }
                if kind == "claim"
                    && attrs
                        .get("polarity")
                        .is_some_and(|value| value != "positive" && value != "negative")
                {
                    return Err(vec![
                        Diagnostic::error(
                            "importer.markdown.polarity-invalid",
                            "polarity must be positive or negative",
                        )
                        .with_source(revision.source_id.clone()),
                    ]);
                }
                nodes.push(Node {
                    start: offset,
                    end: offset + line.len(),
                    kind: kind.into(),
                    attrs,
                });
            }
            offset += line.len();
        }
        Ok(MarkdownDocument {
            revision: revision.clone(),
            text,
            nodes,
        })
    }

    fn emit(&self, doc: MarkdownDocument, ir: &mut SemanticIr) -> Result<(), Vec<Diagnostic>> {
        let mut errors = Vec::new();
        // Preserve the complete source, including constructs with no semantic output.
        let whole = SourceRegion::utf8(
            &doc.revision,
            &doc.text,
            0,
            doc.text.len(),
            Some("document".into()),
        )
        .unwrap();
        push_region(ir, &whole);
        for (index, node) in doc.nodes.iter().enumerate() {
            let region = SourceRegion::utf8(
                &doc.revision,
                &doc.text,
                node.start,
                node.end,
                Some(format!("annotation-{index}")),
            )
            .unwrap();
            push_region(ir, &region);
            let provenance = vec![Provenance::region(&region)];
            let required = |key: &str| {
                node.attrs.get(key).cloned().ok_or_else(|| {
                    Diagnostic::error(
                        "importer.markdown.attribute-missing",
                        format!("{} annotation requires {key}", node.kind),
                    )
                    .with_region(region.id.clone())
                })
            };
            let result: Result<(), Diagnostic> = (|| {
                match node.kind.as_str() {
                    "entity" => {
                        let id = EntityId::new(required("id")?).map_err(id_diag)?;
                        let kind = node.attrs.get("kind").cloned();
                        if let Some(existing) = ir.entities.iter_mut().find(|value| value.id == id)
                        {
                            if existing.kind != kind {
                                return Err(Diagnostic::error(
                                    "importer.markdown.entity-conflict",
                                    format!("entity {id} has conflicting kinds"),
                                ));
                            }
                            if let Origin::Explicit { provenance: values } = &mut existing.origin {
                                values.extend(provenance);
                            }
                        } else {
                            ir.entities.push(Entity {
                                id,
                                kind,
                                origin: Origin::Explicit { provenance },
                                extensions: BTreeMap::new(),
                            });
                        }
                    }
                    "claim" => {
                        let subject = EntityId::new(required("subject")?).map_err(id_diag)?;
                        let predicate = required("predicate")?;
                        let value = Value::Text(required("value")?);
                        let polarity =
                            if node.attrs.get("polarity").is_some_and(|v| v == "negative") {
                                Polarity::Negative
                            } else {
                                Polarity::Positive
                            };
                        let id = record_id(node, &region, "claim")?;
                        let assertion_revision =
                            assertion_revision(&(&subject, &predicate, &value, &polarity));
                        ir.claims.push(Claim {
                            id,
                            assertion_revision,
                            subject,
                            predicate,
                            value,
                            polarity,
                            origin: Origin::Explicit { provenance },
                        });
                    }
                    "relation" => {
                        let kind = required("kind")?;
                        let from =
                            Endpoint::Entity(EntityId::new(required("from")?).map_err(id_diag)?);
                        let to = Endpoint::Entity(EntityId::new(required("to")?).map_err(id_diag)?);
                        let id = record_id(node, &region, "relation")?;
                        let assertion_revision = assertion_revision(&(&kind, &from, &to));
                        ir.relations.push(Relation {
                            id,
                            assertion_revision,
                            kind,
                            from,
                            to,
                            origin: Origin::Explicit { provenance },
                        });
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })();
            if let Err(error) = result {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn record_id(
    node: &Node,
    region: &SourceRegion,
    kind: &str,
) -> Result<SemanticRecordId, Diagnostic> {
    match node.attrs.get("id") {
        Some(id) => named_record_id(id).map_err(id_diag),
        None => Ok(explicit_record_id(&region.id, kind, "0")),
    }
}
fn id_diag(error: IdError) -> Diagnostic {
    Diagnostic::error("importer.markdown.id-invalid", error.to_string())
}
fn push_region(ir: &mut SemanticIr, region: &SourceRegion) {
    ir.source_regions.push(SemanticSourceRegion {
        id: region.id.clone(),
        source_id: region.source_id.clone(),
        revision_id: region.revision_id.clone(),
    });
}
fn parse_attrs(mut input: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    while !input.trim().is_empty() {
        input = input.trim_start();
        let eq = input
            .find('=')
            .ok_or_else(|| "expected key=\"value\"".to_string())?;
        let key = &input[..eq];
        input = &input[eq + 1..];
        if key.is_empty() || !input.starts_with('"') {
            return Err("expected key=\"value\"".into());
        }
        input = &input[1..];
        let end = input
            .find('"')
            .ok_or_else(|| "unterminated quoted value".to_string())?;
        if out.insert(key.into(), input[..end].into()).is_some() {
            return Err(format!("duplicate attribute {key}"));
        }
        input = &input[end + 1..];
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn knowledge_importers_markdown_requires_explicit_annotations() {
        let bytes = b"# prose\nThe system is fast.\n".to_vec();
        let revision = SourceRevision::from_bytes(
            SourceId::new("source.doc").unwrap(),
            "text/markdown",
            &bytes,
            BTreeMap::new(),
        );
        let importer = MarkdownImporter;
        let syntax = importer.parse(&revision, bytes).unwrap();
        let mut ir = SemanticIr::new(KnowledgeSetId::new("markdown.test").unwrap());
        importer.emit(syntax, &mut ir).unwrap();
        assert!(ir.entities.is_empty() && ir.claims.is_empty());
        assert_eq!(ir.source_regions.len(), 1);
    }
}
