# Authored source and generated channel shape

**Status:** accepted publication contract.

## Authored source

```text
source/
├── src/                 Rust workspace
├── assets/              authored shared visual assets
├── releases/            authored release records and capture configuration
├── i18n/                shared locale registry and translation data
├── pages/               Pages templates, configuration, and build.luau
├── repo/                public repository templates, configuration, and build.luau
├── .github/
│   ├── actions/         GitHub-specific operation implementations
│   └── workflows/       triggers and job topology
├── .kero/               KERO knowledge/configuration for the source repository
└── repository files describing the source repository
```

There is no generic `automation/`, `scripts/`, `builders/`, or `content/`
domain. Each major output system owns its implementation. Shared localization
belongs to `i18n/`; GitHub-specific behavior belongs to `.github/actions/`.

The Rust workspace remains under `src/`. A `src/crates/` layout is the target
only when independently justified crate/API boundaries exist; directory shape
must not pretend those boundaries exist early.

## Generated channels

```text
canary | beta | stable
├── docs/<locale>/       finished translated documentation
├── assets/              public asset projection
├── releases/            public release history
├── src/                 public Rust source projection
├── .github/             public-repository GitHub files only
├── README.md
├── README.<locale>.md
├── CONTRIBUTING.md
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── CITATION.cff
├── LICENSE
└── publication.toml
```

`src/` keeps the same structural meaning across the generation boundary.
Flattening Cargo files into the generated branch root requires a later accepted
decision backed by a concrete ecosystem requirement.

`publication.toml` records at least:

```toml
channel = "stable"
version = "2026.08.4-hotfix"
source = "a909851721f709b05a8f58e93e96b6972f2fe6af"
```

A timestamp is added only if a consumer needs it; the source commit remains the
reproducible provenance anchor.

## Generation boundary

Direct projections keep their domain name:

```text
source/src/       -> channel/src/
source/assets/    -> channel/assets/
source/releases/  -> channel/releases/
```

Transforms combine authored inputs into finished outputs:

```text
source/i18n/ + source/pages/  -> channel/docs/
source/i18n/ + source/repo/   -> channel root documents
```

Generated channels do not contain `i18n/`, `pages/`, `repo/`, source `.kero/`,
or source-side `.github/actions/` implementations. These are construction
inputs rather than public outputs.

## Why

Domain ownership stays visible on both sides of publication. Directly projected
content keeps its meaning, transformed content becomes the public artifact it
actually represents, and build machinery does not leak into the release merely
because it contributed to that artifact.
