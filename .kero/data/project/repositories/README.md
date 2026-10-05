# Repository model

kero-net separates canonical source, public release surfaces, and
organization-wide GitHub defaults into independent repositories.

| Repository | Role |
|---|---|
| `kero-net/src` | authoritative implementation and publication inputs |
| `kero-net/kero` | generated public release channels, packages, issues, and discussions |
| `kero-net/.github` | organization profile, templates, and reusable workflow assets |

## Decision

Generated channel content does not live in the authoritative source history.
Public collaboration happens on the release-facing `kero` repository, while
implementation history remains in `src`.

## Why

Source and generated release surfaces have different ownership and history.
Separating them keeps generated channel commits from obscuring implementation
history while still giving releases a stable public repository for issues,
discussions, packages, and documentation.

## Consequences

Changes to source, release, and organization repositories use their own Git
histories. The local `kero-net/` parent remains only a coordination container.
