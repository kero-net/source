# Distribution

This directory owns desktop-package facts and mechanics: target definitions,
toolchain requirements, package assembly, checksums, detached signatures, and
artifact verification. It does not select repository channels or publish.

`builds.json` is the human-owned target contract. `scripts/` runs
that contract locally; generated packages, logs, and evidence remain below
`.heap/`.

The KST registries are the sole editable target and toolchain sources.
`Cargo.toml` and other external-tool manifests are outside this migration.

Each target declares its matching native host and toolchain. A package build
must use that exact pairing; compiler families, Qt kits, and runtime deployment
files are not interchangeable across targets. Local paths are contributor
environment configuration and are never committed.

Repository release records and canary/beta/stable publication remain under
`releases/` because they are a separate repository-release subsystem.

## Local native validation

`distribution/tests/` owns package-contract, architecture, and startup checks.
`kero build TARGET` creates and statically validates an artifact. `kero verify
TARGET` repeats static validation. `kero test TARGET --adapter NAME` adds the
bounded runtime check and records whether execution was native, emulated, or a
VM. Cross-build output never becomes native release evidence.

On Windows, run `./kero.cmd build current`; on Linux, run `./kero build
current`. These action wrappers locate the local target toolchain and invoke
Lua internally. The Linux wrapper discovers `lua`/`lua5.4`, Qt through
`qtpaths6`, and `linuxdeploy` from `PATH`, with environment-variable overrides
for nonstandard installations. `portable` runs portable contracts and, when installed, the
local `act` workflow. `all` records the available local
coverage and marks unavailable native targets as partial rather than release
evidence. `verify TARGET` validates a package already under `.heap/`.

`kero all current` is the standard local validation run: it starts with shared
source checks, then portable checks, then builds/tests every target the current
host can validate. Missing native toolchains or hosts are recorded in
`.heap/distribution/summary.toml` as partial coverage; failed available checks
still fail the command.

For a Windows ARM64 package, the wrapper requires the Qt `msvc2022_arm64` kit
and the matching Visual Studio ARM64 target compiler. It refuses to use an x64
compiler for that package because the resulting bootstrap and DLLs would not
be native ARM64.

`propose` requires a clean committed revision and native runtime evidence for
every enabled target. It
writes a reviewable candidate manifest under `distribution/candidates/`; the
packages themselves remain local. Maintainers reproduce approved candidates on
native hosts and sign only from their protected local key environment.

Contributor commands never require credentials, tokens, or signing keys. A
package is unsigned by default even if a key fingerprint exists in the shell.
Only GitHub's protected `release` environment sets `KERO_SIGN_RELEASE=1` and
receives the release signing key. It publishes the signed packages; local and
pull-request outputs remain deliberately unsigned.

Windows ARM64 can cross-build the configured LLVM-MinGW Windows x64 target.
`kero test windows-x64 --adapter emulated` is opt-in on Windows ARM64 and is
recorded as emulated evidence. Optional Hyper-V and QEMU settings belong in
ignored `distribution.local.kst`; copy `distribution.local.kst.example` to
start. QEMU never downloads guest images or credentials.

macOS ARM64 is currently **coming soon**, not enabled release coverage. A DMG
requires Apple tools and a matching Qt kit on a Mac contributor host.
