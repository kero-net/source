# Documentation Standards

SCOPE-owned repository knowledge lives under `.scope/knowledge/shared/`.
Machine-local knowledge lives under `.scope/knowledge/local/` and is ignored.
SCOPE does not prescribe an ordinary repository's `docs/` layout. In this
repository only, publication automation replaces the source `docs/README.md`
with a localized channel-facing README rendered from `content/repo/`. Pages and
future wiki content live under `content/pages/`; only that tree defines the
public site structure consumed by MkDocs.

## Role Terminology

Use role-based language consistently:

<table>
  <tr><td><strong>user</strong></td><td>Uses the project or its tools</td></tr>
  <tr><td><strong>contributor</strong></td><td>Modifies or contributes to the project</td></tr>
  <tr><td><strong>maintainer</strong></td><td>Manages the project, including administration and ownership</td></tr>
</table>

Describe repository surfaces by purpose. Name the relevant role when a
distinction is required.

Prefer present tense, active voice, direct requirements, and canonical links.
Use “must” for requirements, “should” for recommendations, and “can” for
optional behavior. Format paths, commands, identifiers, and literals as code.

Use HTML tables for repeated mappings or comparable records, such as path and
purpose, role and responsibility, or setting and effect. Omit table headers
unless the column meanings require labels. Use bullet lists for simple unordered
sets and numbered lists for ordered procedures.

Keep examples short and executable. Do not document speculative APIs or
workflows as implemented behavior.

MkDocs writes only to `.generated/site/`. Never commit generated site output.
