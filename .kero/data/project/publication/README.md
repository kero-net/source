# Publication

`kero-net/src:source` is the authoritative publication input. Publication
produces generated channel branches in `kero-net/kero`:

```text
src:source  -->  kero:canary
            -->  kero:beta
            -->  kero:stable
```

Publication is validated first using outputs beneath `kero-net/source/.heap/`.
The public branches are generated and must not be hand-edited.

## Decision

All generated channels use one path schema. Channel selection may change
versions, content, freshness, and provenance, but not the structural meaning of
a path.

## Why

Consumers should not need different repository logic for `canary`, `beta`, and
`stable`. Keeping the shape fixed makes channel switching a content/provenance
choice rather than a repository-layout migration.

## Consequences

Construction inputs such as templates, localization source, Cargo behavior,
and source-side GitHub action implementations remain in `src:source` rather
than being copied into generated branches.

See [`generated-shape.md`](generated-shape.md) for the authored and generated
schemas.
