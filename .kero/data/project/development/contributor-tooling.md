# Contributor tooling

**Status:** active.

The source repository must own the shared VS Code contributor experience for
Rust, Qt/C++, CMake, and Lua work. Opening the authoritative `source/`
repository should expose the appropriate extension recommendations and the
safe, portable build entry points without requiring a contributor to discover
local coordination-folder configuration.

## Current state

`source/.vscode/extensions.json` recommends Rust Analyzer, C/C++, CMake Tools,
and Lua support. `.heap/` is ignored within `source/` so disposable output
follows the repository when it is opened directly. VS Code exposes one standard
task, `Kero: Validate Locally`, which calls the source-owned runner.

## Direction

Local native jobs own package execution. `kero all current` runs shared source
checks, portable checks, and all locally available package/runtime checks before
writing evidence under `.heap/`, without GitHub CLI, remote staging, or credentials.
It separates cross-build output from runtime evidence and records unavailable
native coverage as partial. `act` provides optional portable/Linux container
checks; native Windows and macOS checks run only on matching local hosts or
configured local VMs. Target-specific toolchain requirements remain canonical under
[`product/distribution/`](../../product/distribution/).

The Windows and POSIX `kero` wrappers are equivalent contributor entry points.
The Linux wrapper resolves Lua, Qt through `qtpaths6`, and `linuxdeploy` from
the local distribution, then reports missing packages without relying on
contributor-specific committed paths. `act` runs the same portable Lua stage
inside a Linux container when both it and Docker are installed. The repository
`.actrc` supplies the shared image mapping but no secrets, tokens, or automatic
installation; it never substitutes for native desktop evidence.

## Configuration ownership

New human-editable KERO-owned operational settings use KERO Structured Text
(KST). Every mutable default, policy, toolchain, or release value has one
authoritative editable definition; scripts and generated records consume or
derive it rather than carrying a second default. Migrate existing KERO-owned
TOML settings incrementally with changes to their owning subsystem. Keep TOML
only where an external tool requires it, including Cargo manifests.

The current ownership inventory and migration sequence are recorded in
[`kst-operational-settings.md`](kst-operational-settings.md).

Review changes for configuration ownership and add focused contract coverage
for each migrated setting schema and consumer. Do not use a generic duplicate
literal scan: matching text is not proof of duplicate policy.

## Replacement record

This document replaces the implicit assumption that local VS Code tasks are
the package entry point and the prior coupling of every build to a runtime
test. The source repository is the actual clone/open boundary; its remote
pipeline and disposable output live there.

## Next evidence

Verify a clean clone can make an unsigned current-host package, record partial
coverage accurately, and reject a candidate proposal until a clean commit and
complete native evidence exist.
