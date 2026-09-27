# KST operational settings migration

**Status:** complete for operational settings.

KERO Structured Text is the sole target format for KERO-owned, human-editable
operational settings. This document records the migration work; the durable
format and ownership contract remains under
[`product/environment/configuration/`](../../product/environment/configuration/).

## Ownership inventory

| Setting family | Current owner | Current consumers | Migration order |
|---|---|---|---|
| distribution targets and signing-variable names | `distribution/distribution.kst` | package, local-run, validation scripts, workflow checks | complete |
| contributor VM adapters | ignored `distribution.local.kst` | adapter, Hyper-V, local-run scripts | complete |
| toolchain labels | `distribution/toolchains.kst` | distribution documentation/tooling | complete |
| repository publication settings | `repo/config.kst` | repository build and publication action | complete |
| localization catalogs | `i18n/*.toml` | renderer, validator, publication checks | separate content-schema migration |

Generated evidence, candidate manifests, and publication records do not become
human-owned configuration merely because they are serialized text. Their format
is chosen by their owning output contract.

Cargo manifests, CMake presets, and other third-party tool manifests remain in
the formats required by those tools.

## First migration: distribution targets

`distribution/distribution.kst` owns the target registry and signing-variable
names. One shared Lua reader serves package, orchestration, key validation,
toolchain validation, and contract tests; there is no TOML fallback or duplicate
target default in scripts.

The migration is complete only when the KST registry is the sole editable
distribution-target source and all listed consumers reject malformed, missing,
or contradictory KST values with actionable errors.

All operational setting families in this inventory now use KST. Localization
catalogs remain TOML because they are authored content schemas rather than
operational configuration; replacing them requires a separate content-format
decision and migration.
