# KST operational settings migration

**Status:** superseded for distribution targets by the JSON build registry.

KERO Structured Text is the sole target format for KERO-owned, human-editable
operational settings. This document records the migration work; the durable
format and ownership contract remains under
[`product/environment/configuration/`](../../product/environment/configuration/).

## Ownership inventory

| Setting family | Current owner | Current consumers | Migration order |
|---|---|---|---|
| distribution targets, hosted build inputs, and signing-variable names | `distribution/builds/*/build.toml`, `distribution/tools/*.toml`, and `distribution/policy.toml` | the complete distribution build and hosted workflows | complete |
| repository publication settings | `repo/config.kst` | repository build and publication action | complete |
| localization catalogs | `i18n/*.toml` | renderer, validator, publication checks | separate content-schema migration |

Generated evidence, candidate manifests, and publication records do not become
human-owned configuration merely because they are serialized text. Their format
is chosen by their owning output contract.

Cargo manifests, CMake presets, and other third-party tool manifests remain in
the formats required by those tools.

## Distribution target registry

The TOML manifest tree under `distribution/builds/` replaces `distribution/distribution.kst` and
`distribution/toolchains.kst`. It owns each target's artifact, native host,
local requirements, hosted runner, Qt kit/version, compiler architecture, and
signing-variable names. One shared Lua reader serves package, orchestration,
key validation, toolchain validation, the generated GitHub Actions matrix, and
contract tests; there is no fallback registry or duplicate workflow matrix.

This replaces the earlier KST-only decision because hosted packaging introduced
public, per-distribution inputs that both local tooling and workflow matrices
must consume. JSON makes the complete build shape visible in one place while
the GitHub Actions pin remains in workflow YAML for Dependabot.

Localization catalogs remain TOML because they are authored content schemas.
