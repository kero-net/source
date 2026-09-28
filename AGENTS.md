# KERO repository guidance

KERO is a knowledge-environment product. Its portable `kero.wasm` core owns
KERO semantics; native hosts supply scoped platform capabilities and the Qt
host provides the current desktop presentation and installation layer. This
repository is the authoritative source for that product and for the generated
release inputs.

## Knowledge routing

Before changing KERO behavior, plans, build workflow, or documentation, read
[`.kero/README.md`](.kero/README.md) and then the relevant canonical node under
`.kero/data/`.

- `.kero/data/product/` contains durable product behavior and contracts.
- `.kero/data/project/` contains repository topology, contributor workflow,
  publication, migration state, and implementation progress.
- `.kero/data/standards/` contains knowledge-tree and repository conventions.

Do not use an old plan, a roadmap checklist, generated output, or an inferred
implementation detail as the product contract when a canonical product node
exists. Roadmap files record progress and evidence only. Update the canonical
knowledge in the same change when implementation changes a documented contract.

## Dynamic notes and replaced decisions

Put dynamic work at the narrowest semantic concept it concerns. Name files for
their subject, never for a date. Keep the current state in one canonical
document; do not create competing "new" and "old" plans.

When an idea, plan, or contract is replaced, revise the current document to
identify what it replaces, why the decision changed, and its consequences. If
the prior document remains, mark it superseded and link to its replacement.
Use dates only inside content when they clarify a material event, never as the
identity of a note or a substitute for continuity.

## Repository boundaries

`kero-net/` is a local coordination container. `source/` is the independently
versioned authoritative source repository. Keep durable source-owned work here;
keep disposable output under `.heap/`; do not treat generated release channels
as hand-authored source.

Use source-owned contributor configuration for shared editor/build workflow.
Machine-specific toolchain paths, credentials, and generated state must remain
outside committed source configuration.
