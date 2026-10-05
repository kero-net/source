//! KERO Structured Text (KST) syntax.
//!
//! This module owns only UTF-8 text parsing, source locations, and a
//! deterministic syntax writer. Schema validation and every product policy
//! are intentionally deferred to later passes.

use std::fmt;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub name: String,
    pub value: Option<String>,
    pub children: Vec<Node>,
    pub location: Location,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trivia {
    pub line: usize,
    pub kind: TriviaKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TriviaKind {
    Blank,
    Comment(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Document {
    pub nodes: Vec<Node>,
    pub trivia: Vec<Trivia>,
    pub newline: Newline,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Newline {
    #[default]
    Lf,
    Crlf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigError {
    pub code: &'static str,
    pub message: String,
    pub path: Option<PathBuf>,
    pub location: Option<Location>,
}

impl ConfigError {
    fn syntax(code: &'static str, line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            path: None,
            location: Some(Location { line, column }),
        }
    }
    pub fn at_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.path, &self.location) {
            (Some(path), Some(location)) => write!(
                f,
                "{}:{}:{}: error[{}]: {}",
                path.display(),
                location.line,
                location.column,
                self.code,
                self.message
            ),
            (_, Some(location)) => write!(
                f,
                "{}:{}: error[{}]: {}",
                location.line, location.column, self.code, self.message
            ),
            _ => write!(f, "error[{}]: {}", self.code, self.message),
        }
    }
}
impl std::error::Error for ConfigError {}

/// Parses KST without assigning semantic meaning to names or values.
pub fn parse(input: &str) -> Result<Document, ConfigError> {
    let newline = if input.contains("\r\n") {
        Newline::Crlf
    } else {
        Newline::Lf
    };
    if input.contains('\r') && input.replace("\r\n", "").contains('\r') {
        return Err(ConfigError::syntax(
            "config.lexical.newline",
            1,
            1,
            "bare carriage return is not a supported line ending",
        ));
    }
    let mut flat = Vec::<(usize, Node)>::new();
    let mut trivia = Vec::new();
    let lines = input.split('\n').collect::<Vec<_>>();
    for (index, raw) in lines.iter().enumerate() {
        let line = index + 1;
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if raw.is_empty() && index + 1 == lines.len() && input.ends_with('\n') {
            continue;
        }
        if raw.is_empty() || raw.chars().all(|c| matches!(c, ' ' | '\t')) {
            trivia.push(Trivia {
                line,
                kind: TriviaKind::Blank,
            });
            continue;
        }
        let tabs = raw.bytes().take_while(|byte| *byte == b'\t').count();
        let rest = &raw[tabs..];
        if rest.starts_with(' ') {
            return Err(ConfigError::syntax(
                "config.lexical.space-indent",
                line,
                tabs + 1,
                "spaces are not structural indentation",
            ));
        }
        if let Some(comment) = rest.strip_prefix('#') {
            trivia.push(Trivia {
                line,
                kind: TriviaKind::Comment(comment.to_owned()),
            });
            continue;
        }
        if let Some((previous, _)) = flat.last() {
            if tabs > previous + 1 {
                return Err(ConfigError::syntax(
                    "config.structure.indent-jump",
                    line,
                    1,
                    "child indentation may increase by exactly one level",
                ));
            }
        } else if tabs != 0 {
            return Err(ConfigError::syntax(
                "config.structure.indent-parent",
                line,
                1,
                "indented node has no parent",
            ));
        }
        let (name, value) = parse_line(rest, line, tabs + 1)?;
        flat.push((
            tabs,
            Node {
                name,
                value,
                children: Vec::new(),
                location: Location {
                    line,
                    column: tabs + 1,
                },
            },
        ));
    }
    let mut position = 0;
    Ok(Document {
        nodes: build_nodes(&flat, &mut position, 0),
        trivia,
        newline,
    })
}

/// Writes a machine-owned canonical KST representation using LF newlines.
/// Comments and blank lines remain in `Document::trivia` and are omitted.
pub fn write_canonical(document: &Document) -> String {
    let mut output = String::new();
    write_nodes(&mut output, &document.nodes, 0);
    output
}

fn write_nodes(output: &mut String, nodes: &[Node], depth: usize) {
    for node in nodes {
        output.push_str(&"\t".repeat(depth));
        output.push_str(&node.name);
        if let Some(value) = &node.value {
            output.push(' ');
            if is_bare_value(value) {
                output.push_str(value);
            } else {
                write_quoted(output, value);
            }
        }
        output.push('\n');
        write_nodes(output, &node.children, depth + 1);
    }
}
fn write_quoted(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character => output.push(character),
        }
    }
    output.push('"');
}
fn build_nodes(flat: &[(usize, Node)], position: &mut usize, depth: usize) -> Vec<Node> {
    let mut output = Vec::new();
    while *position < flat.len() && flat[*position].0 == depth {
        let mut node = flat[*position].1.clone();
        *position += 1;
        if *position < flat.len() && flat[*position].0 > depth {
            node.children = build_nodes(flat, position, depth + 1);
        }
        output.push(node);
    }
    output
}
fn parse_line(
    rest: &str,
    line: usize,
    column: usize,
) -> Result<(String, Option<String>), ConfigError> {
    let (name, value) = match rest.split_once(' ') {
        Some((name, value)) => (name, Some(value)),
        None => (rest, None),
    };
    if !is_name(name) {
        return Err(ConfigError::syntax(
            "config.lexical.name",
            line,
            column,
            "node names must be lowerCamelCase ASCII identifiers",
        ));
    }
    Ok((
        name.into(),
        value
            .map(|value| parse_value(value, line, column + name.len() + 1))
            .transpose()?,
    ))
}
fn is_name(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(character) if character.is_ascii_lowercase())
        && characters.all(|character| character.is_ascii_alphanumeric())
}
fn parse_value(value: &str, line: usize, column: usize) -> Result<String, ConfigError> {
    if value.starts_with('"') {
        return parse_quoted(value, line, column);
    }
    if !is_bare_value(value) {
        return Err(ConfigError::syntax(
            "config.lexical.value",
            line,
            column,
            "invalid bare value",
        ));
    }
    Ok(value.into())
}
fn is_bare_value(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !value
            .chars()
            .any(|character| character == '"' || character == '\\' || character.is_control())
}
fn parse_quoted(value: &str, line: usize, column: usize) -> Result<String, ConfigError> {
    let mut output = String::new();
    let mut escaped = false;
    let mut closed_at = None;
    for (offset, character) in value.char_indices().skip(1) {
        if escaped {
            match character {
                '\\' => output.push('\\'),
                '"' => output.push('"'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                _ => {
                    return Err(ConfigError::syntax(
                        "config.lexical.escape",
                        line,
                        column + offset,
                        "invalid quoted-value escape",
                    ));
                }
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            closed_at = Some(offset + character.len_utf8());
            break;
        } else {
            output.push(character);
        }
    }
    let Some(end) = closed_at else {
        return Err(ConfigError::syntax(
            if escaped {
                "config.lexical.escape"
            } else {
                "config.lexical.quote"
            },
            line,
            column,
            if escaped {
                "unterminated quoted-value escape"
            } else {
                "unterminated quoted value"
            },
        ));
    };
    if end != value.len() {
        return Err(ConfigError::syntax(
            "config.lexical.trailing",
            line,
            column + end,
            "unexpected text after quoted value",
        ));
    }
    Ok(output)
}

/// The schema-neutral default KERO Structured Text configuration.
///
/// Repository configuration owns per-mount refresh overrides while global
/// preferences remain in the selected KERO home. The template prevents
/// machine-local service details from becoming portable repository policy.
pub fn default_config() -> &'static str {
    "# KERO repository environment configuration.\n\
#\n\
# This is KERO Structured Text (KST). It holds portable per-mount policy;\n\
# source locations, grants, runtime status, and service details never belong here.\n\
#\n\
# Per-mount refresh override. The last matching mount section is active.\n\
# refresh values: manual | event\n\
# mount global\n\
#   refresh event\n\
# mount global\n\
#   refresh manual\n\
#\n\
# Writable access is not configured here. It requires a signed source-owned\n\
# syncGrant in the source environment configuration and `kero-host mount sync`.\n\
# KERO_HOME, install paths, PATH entries, service ports, and source paths are\n\
# host-local facts and must not be committed in this repository.\n"
}
