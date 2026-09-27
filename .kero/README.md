# KERO knowledge hub

This directory is the source repository's KERO knowledge environment. It is
the first place a contributor or agent should read to understand what KERO is,
where a fact belongs, and which work is current.

```text
.kero/
├── config       human-owned environment configuration
├── data/        durable, versioned knowledge
│   ├── product/     KERO behavior and contracts
│   ├── project/     kero-net workflow, implementation, and progress
│   └── standards/   rules for this knowledge and repository ownership
└── mnt/         reserved isolated external KERO environments
```

`.runtime/` is intentionally absent until a host needs it. It is hidden,
disposable runtime state, never durable knowledge.

## Start here

Read [`data/README.md`](data/README.md), then the branch relevant to the
work:

- `data/product/` for product semantics, environment behavior, host contracts,
  and distribution requirements;
- `data/project/` for repository ownership, publication, contributor tooling,
  migrations, and roadmap state;
- `data/standards/` for placement, naming, and generated-state rules.

The tree grammar and the distinction between durable product truth and project
state are defined in [`data/standards/knowledge-tree.md`](data/standards/knowledge-tree.md).

## Dynamic work and replacement

Keep the current truth at the narrowest concept where it is true. A live
investigation, implementation plan, or contributor note belongs beneath that
concept in `data/product/` or `data/project/`; it is not a separate dated
archive and it must not be placed in `.runtime/`.

Name a note for its subject, not the day it was written. For example,
`contributor-tooling.md` is preferable to `2026-09-22-notes.md`. A branch may
use a `notes/` directory only when the notes are genuinely children of one
specific concept; its `README.md` explains their shared scope.

When new reasoning replaces an earlier plan or contract, update the canonical
document rather than leaving two competing current documents. Preserve the
important bridge in the replacement document: state what it replaces, why it
changed, and the consequence. A superseded document must say where its current
replacement lives. Dates may clarify a material event in content, but they are
not note identities or a substitute for that explicit replacement link.
