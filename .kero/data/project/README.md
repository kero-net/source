# kero-net project

This branch describes the kero-net workspace and the work required to build and
publish KERO. It is project knowledge, not KERO product semantics.

The local workspace is a coordination container rather than a Git repository:

```text
kero-net/
├── .github/      github.com/kero-net/.github
├── source/       github.com/kero-net/src
├── kero/         github.com/kero-net/kero
└── source/.heap/ ignored, fully disposable build and publication output
```

Each direct repository child is independently versioned. `source/.heap/` is
deleted and recreated by a full build, and is never a source of truth. Its
canonical generated shape is owned by
[`development/contributor-tooling.md`](development/contributor-tooling.md).

- [`repositories/`](repositories/) defines repository ownership.
- [`publication/`](publication/) defines source-to-channel publication.
- [`development/`](development/) defines local tooling and implementation
  progress.

## Why

Separating project mechanics from product truth prevents build tooling,
repository topology, temporary migrations, or roadmap status from becoming
accidental KERO behavior.
