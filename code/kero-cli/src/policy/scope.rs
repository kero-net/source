use super::model::ScopeValue;
use thiserror::Error;

const TYPES: &[&str] = &[
    "branch",
    "command",
    "environment",
    "knowledge",
    "path",
    "repository",
    "service",
];

pub(super) fn type_known(value: &str) -> bool {
    TYPES.contains(&value)
}

#[derive(Debug, Error)]
pub enum ScopeError {
    #[error("scope.invalid: {0}")]
    Invalid(String),
    #[error("scope.type-unknown: {0}")]
    UnknownType(String),
}

pub fn parse_scope(raw: &str) -> Result<ScopeValue, ScopeError> {
    let (kind, expression) = raw
        .split_once(':')
        .ok_or_else(|| ScopeError::Invalid(raw.into()))?;
    if !TYPES.contains(&kind) {
        return Err(ScopeError::UnknownType(kind.into()));
    }
    let expression = normalize(kind, expression)?;
    Ok(ScopeValue {
        kind: kind.into(),
        expression,
    })
}

fn normalize(kind: &str, expression: &str) -> Result<String, ScopeError> {
    if expression.is_empty() {
        return Err(ScopeError::Invalid(expression.into()));
    }
    if kind == "path" {
        let value = expression.replace('\\', "/");
        if value.starts_with('/')
            || value.as_bytes().get(1) == Some(&b':')
            || value.split('/').any(|part| {
                part.is_empty()
                    || matches!(part, "." | "..")
                    || !part.chars().all(|character| {
                        character.is_ascii_alphanumeric()
                            || matches!(character, '.' | '_' | '+' | '@' | '*' | '-')
                    })
            })
        {
            return Err(ScopeError::Invalid(expression.into()));
        }
        return Ok(value);
    }
    if expression.contains("**")
        || !expression.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '_' | '/' | '@' | '+' | '*' | '-')
        })
        || (expression.contains('*') && expression != "*" && !expression.ends_with(".*"))
    {
        return Err(ScopeError::Invalid(expression.into()));
    }
    Ok(expression.into())
}

pub fn scope_matches(constraint: &ScopeValue, target: &ScopeValue) -> Option<bool> {
    if constraint.kind != target.kind {
        return Some(false);
    }
    let expression = &constraint.expression;
    let target = &target.expression;
    if expression == "*" || (constraint.kind == "path" && expression == "**") {
        return Some(true);
    }
    if !expression.contains('*') {
        return Some(expression == target);
    }
    if constraint.kind == "path" {
        if let Some(prefix) = expression.strip_suffix("/**") {
            if !prefix.contains('*') {
                return Some(target == prefix || target.starts_with(&format!("{prefix}/")));
            }
        }
        return None;
    }
    if let Some(prefix) = expression.strip_suffix(".*") {
        return Some(target == prefix || target.starts_with(&format!("{prefix}.")));
    }
    None
}

pub fn scope_contains(outer: &ScopeValue, inner: &ScopeValue) -> Option<bool> {
    if outer.kind != inner.kind {
        return Some(false);
    }
    if outer == inner
        || outer.expression == "*"
        || (outer.kind == "path" && outer.expression == "**")
    {
        return Some(true);
    }
    if !inner.expression.contains('*') {
        return scope_matches(outer, inner);
    }
    if outer.kind == "path" {
        if let (Some(parent), Some(child)) = (
            outer.expression.strip_suffix("/**"),
            inner.expression.strip_suffix("/**"),
        ) {
            if !parent.contains('*') && !child.contains('*') {
                return Some(child == parent || child.starts_with(&format!("{parent}/")));
            }
        }
    } else if let (Some(parent), Some(child)) = (
        outer.expression.strip_suffix(".*"),
        inner.expression.strip_suffix(".*"),
    ) {
        return Some(child == parent || child.starts_with(&format!("{parent}.")));
    }
    None
}

pub fn scope_set_match(
    constraints: &[ScopeValue],
    universe: &[String],
    targets: &[ScopeValue],
) -> Option<bool> {
    // An omitted scope set means the record is unconstrained. Once a record
    // declares either constraints or a universe, every target type must be
    // covered explicitly; silently ignoring a second target type could widen
    // an authorization decision.
    if constraints.is_empty() && universe.is_empty() {
        return Some(true);
    }
    let mut result = Some(true);
    for target in targets {
        if universe.contains(&target.kind) {
            continue;
        }
        let outcomes: Vec<Option<bool>> = constraints
            .iter()
            .filter(|constraint| constraint.kind == target.kind)
            .map(|constraint| scope_matches(constraint, target))
            .collect();
        if outcomes.contains(&Some(true)) {
            continue;
        }
        if outcomes.contains(&None) {
            result = None;
        } else {
            return Some(false);
        }
    }
    result
}

pub fn scopes_contain(
    outer: &[ScopeValue],
    outer_universe: &[String],
    inner: &[ScopeValue],
    inner_universe: &[String],
) -> Option<bool> {
    if inner_universe
        .iter()
        .any(|kind| !outer_universe.contains(kind))
    {
        return Some(false);
    }
    let mut result = Some(true);
    for child in inner {
        if outer_universe.contains(&child.kind) {
            continue;
        }
        let outcomes: Vec<Option<bool>> = outer
            .iter()
            .filter(|parent| parent.kind == child.kind)
            .map(|parent| scope_contains(parent, child))
            .collect();
        if outcomes.contains(&Some(true)) {
            continue;
        }
        if outcomes.contains(&None) {
            result = None;
        } else {
            return Some(false);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_scope_containment_is_conservative() {
        assert_eq!(
            scope_contains(
                &parse_scope("path:src/**").unwrap(),
                &parse_scope("path:src/parser/**").unwrap()
            ),
            Some(true)
        );
        assert_eq!(
            scope_contains(
                &parse_scope("path:src/parser/**").unwrap(),
                &parse_scope("path:tests/**").unwrap()
            ),
            Some(false)
        );
        assert_eq!(
            scope_contains(
                &parse_scope("path:src/*/generated.rs").unwrap(),
                &parse_scope("path:src/**/generated.rs").unwrap()
            ),
            None
        );
    }

    #[test]
    fn paths_are_abstract_and_cannot_escape() {
        for invalid in ["path:/etc/passwd", "path:../secret", "path:C:\\secret"] {
            assert!(parse_scope(invalid).is_err(), "accepted {invalid}");
        }
        assert_eq!(
            parse_scope("path:src\\main.rs").unwrap().canonical(),
            "path:src/main.rs"
        );
    }

    #[test]
    fn same_type_scope_expressions_are_alternatives() {
        let constraints = [
            parse_scope("path:src/**").unwrap(),
            parse_scope("path:tests/**").unwrap(),
        ];
        assert_eq!(
            scope_set_match(
                &constraints,
                &[],
                &[parse_scope("path:src/main.rs").unwrap()]
            ),
            Some(true)
        );
        assert_eq!(
            scope_set_match(
                &constraints,
                &[],
                &[parse_scope("path:docs/index.md").unwrap()]
            ),
            Some(false)
        );
    }

    #[test]
    fn every_request_target_must_be_covered_by_a_declared_scope() {
        assert_eq!(
            scope_set_match(
                &[parse_scope("path:src/**").unwrap()],
                &[],
                &[
                    parse_scope("path:src/main.rs").unwrap(),
                    parse_scope("repository:example").unwrap(),
                ],
            ),
            Some(false)
        );
        assert_eq!(
            scope_set_match(
                &[parse_scope("path:src/**").unwrap()],
                &["repository".into()],
                &[
                    parse_scope("path:src/main.rs").unwrap(),
                    parse_scope("repository:example").unwrap(),
                ],
            ),
            Some(true)
        );
    }
}
