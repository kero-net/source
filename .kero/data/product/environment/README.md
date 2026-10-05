# KERO environment

A KERO environment separates durable local knowledge, explicitly mounted
external knowledge, human-facing configuration, and disposable runtime state.

A repository environment has this semantic layout:

```text
.kero/
├── config
├── data/
├── mnt/
└── .runtime/       created only when required
```

| Path | Owner | Meaning |
|---|---|---|
| `config` | human | environment policy and product configuration |
| `data/` | local environment | durable local knowledge |
| `mnt/<name>/` | mount host/runtime | isolated materialized external knowledge |
| `.runtime/` | host/runtime | hidden disposable state, locks, caches, and transport data |

Local resolution begins under `data/`. Mounted resolution begins under
`mnt/<name>/`. Neither scope may escape its assigned root or be treated as the
other merely because both contain files.

Filesystem requirements and fail-closed snapshot behavior are defined in
[`filesystem-compatibility.md`](filesystem-compatibility.md).

## Decision

Machine identity, CPU/OS facts, credentials, absolute host paths, and transport
locators do not belong in repository `config`.

## Why

Repository knowledge must have the same meaning when cloned or opened on a
different machine. Host-specific details belong to the host capability that
uses them, not to the portable repository contract.

## Consequences

Runtime state is not knowledge, and global KERO-home state is not ambient input
to a repository. Both must be reached through explicit product or host rules.
