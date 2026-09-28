# KERO local knowledge

This directory is the root of durable knowledge owned by this KERO environment.
Its first-level branches separate three different kinds of truth:

- [`product/`](product/) describes what KERO is and how its product contracts
  work.
- [`project/`](project/) describes the kero-net workspace, repositories,
  publication process, development tooling, and implementation progress.
- [`standards/`](standards/) defines durable rules for how this knowledge and
  its repositories are organized.

## Why

Product behavior, project state, and organizational standards change for
independent reasons. Keeping them as separate root concepts prevents roadmap
status or repository mechanics from becoming accidental product semantics.

Knowledge is placed at the narrowest node where it is true, then inherited by
that node's descendants. The filesystem grammar for that model is defined in
[`standards/knowledge-tree.md`](standards/knowledge-tree.md).
