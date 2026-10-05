# KERO Structured Text

**Status:** syntax contract, version 1.

KERO Structured Text (KST) is UTF-8 text for human-owned KERO configuration and
future declarative documents. It is a syntax only: node names and values have
no built-in serialization, signing, mount, OS, CPU, or transport meaning.
Those meanings require explicit schemas above the syntax layer.

## Lines and tree shape

- LF and CRLF input are accepted; a bare carriage return is rejected.
- A node begins with zero or more literal tabs, then a lowerCamelCase ASCII
  name, and optionally one ASCII space followed by a value.
- One tab is one depth. A child can increase depth by exactly one. Spaces are
  never structural indentation.
- Blank lines and full-line comments are non-semantic trivia. A comment begins
  when `#` is the first character after indentation.
- Duplicate siblings remain in source order; a schema decides whether they are
  valid for a particular document.

For example, this is valid KST structure:

```text
serialization deterministicCbor

compression zstd
	level 3

mount local
	source data
	enabled true

mount shared
	source ../shared-knowledge
	target mnt/shared
	enabled true
```

It parses as a tree with repeated `mount` nodes. It is not TOML, YAML, JSON, or
a generic string-keyed object. The parser fixes this tree grammar; a future
mount schema may choose either `mount name` or nested `mount` / `name` nodes,
but neither form is selected by KST itself.

## Values and diagnostics

Bare values cannot be empty, edge-whitespace, quotes, backslashes, or control
characters. Quoted values use `"` delimiters and only permit `\\`, `\"`,
`\n`, `\r`, and `\t` escapes. An invalid escape, unterminated quote, or text
after a closing quote is lexical failure.

Every syntax diagnostic has a stable code and one-based line/column. Parsing
retains node locations plus comment/blank-line trivia for a human-config
editor. Comments and grouping are first-class human-owned context: an editor
that changes a setting must preserve unaffected comments, blank lines, sibling
order, and tree structure. The canonical writer is deliberately separate: it
produces LF-only, tab-indented node text and omits trivia, so it must not be
used as an implicit rewrite of a human-owned document.

## Format ownership

KST is the sole format for new KERO-owned human-editable operational settings.
Each schema defines its own node meanings, validation, and whether repeated
siblings are permitted. The owning schema must name one authoritative location
for every mutable default; derived output is not another configuration source.

## Why

KST keeps the human configuration format deterministic and structurally simple
without assigning product semantics at the parser layer. Separating the
comment-preserving editor from the canonical writer avoids destroying human
context merely because a tool touched `config`.
