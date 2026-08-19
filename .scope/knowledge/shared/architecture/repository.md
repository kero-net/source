# Repository Architecture

The repository contains every SCOPE-owned project integration surface below
`.scope/`. Ordinary repository documentation remains outside that contract.

```text
source
├── .scope/              project knowledge, policy, tests, and runtime state
├── content/             localization registries, catalogs, and templates
├── docs/                source-branch README; not owned by SCOPE discovery
├── src/                 Rust package and tests
├── .github/             repository policy and publication automation
└── .generated/          ignored local builds and publication payloads
```

SCOPE-specific shared knowledge lives under `.scope/knowledge/shared/` and its
registry is `.scope/knowledge.toml`. Machine-local knowledge belongs under
`.scope/knowledge/local/` and is ignored. SCOPE does not require, search, or
assign semantics to an ordinary repository's `docs/` directory.

In this repository's publication system, `docs/README.md` is the source-branch
README. Automation replaces it on generated branches with localized,
channel-facing repository READMEs rendered from `content/repo/`. That is a
repository publication choice, not a SCOPE knowledge-storage convention.

The `source` branch is canonical. Automation derives `canary`, `beta`, and
`stable` from immutable source commits. Generated branches and `.generated/`
must never be hand-edited.

## Consumer Repository Surface

SCOPE creates only `.scope/` in a repository that adopts it. Project knowledge,
policy, test scenarios, and local runtime state stay below that one boundary.
Its own `.gitignore` excludes `knowledge/local/` and `state/`, so adopting SCOPE
does not require another top-level ignore or state directory.

The SCOPE source repository is different: it retains the Visit-Counter
authoring format because it builds the CLI, localized Pages site, translated
repository READMEs, generated release channels, and GitHub control plane.
