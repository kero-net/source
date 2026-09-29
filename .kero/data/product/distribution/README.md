# Distribution

KERO release artifacts are built from the authoritative source and distributed
per native host target. Platform packaging may differ, but it never changes
KERO core semantics or creates a platform-specific product version.

## Integrity policy

Every package receives a SHA-256 checksum. Contributor packages and pull
request CI artifacts are unsigned by default and never require credentials or
key material. A detached signature is added only in GitHub's protected
`release` environment, which explicitly enables signing after the validated
source revision has been selected for publication. The KERO public GPG key and
its full fingerprint accompany releases that use those signatures.

Platform-native code signing and notarization are currently deferred. KERO does
not simulate those trust signals or claim unsigned artifacts are natively
signed.

## Build and runtime validation

Package creation, static validation, and runtime validation are distinct.
Cross-builds may prove package integrity and architecture but never provide
native release evidence. Every enabled target needs matching native runtime
evidence before a candidate may be proposed. Runtime records state `native`,
`emulated`, or `vm`; an emulated Windows x64 test on Windows ARM64 is useful
diagnostic evidence and is never relabelled native.

Downloaded build toolchains are disposable state under `.heap`, not
machine-local source configuration. A full build starts by deleting `.heap/`
and bootstraps every needed disposable tool again from committed definitions.
Therefore `.heap/` must never be an implicit prerequisite for a later run.

Completed distribution artifacts are assembled together beneath
`.heap/artifacts/distributions/`. `.heap/artifacts/SHA256SUMS` covers that set;
GitHub adds `.heap/artifacts/SHA256SUMS.asc` using protected secrets, while
local builds remain unsigned. `.heap/RELEASE.md` is the generated release
description. Intermediate Rust, Cargo, Qt, and downloaded toolchain data belongs
under `.heap/build/`, separate from completed artifacts.

macOS remains future work until its final target and native evidence contract
are selected.

Human-editable KERO-owned operational settings migrate to KERO Structured Text
incrementally with their owning subsystem. Existing TOML remains in place until
that work occurs and where external tools require it; a format-only migration
is not a distribution change.

## Decision

Current distribution prioritizes reproducible, user-verifiable artifacts
without requiring paid platform code-signing programs.

## Why

Checksums and detached signatures can be produced and verified consistently
across supported platforms while the project is not paying for each platform's
native signing program. This gives contributors a reproducible release path
without conflating GPG verification with OS-native trust UI.

## Consequences

- Windows and macOS may produce development/release artifacts without a GPG key;
  the checksum is still required.
- Linux publication requires the configured GPG signature in addition to the
  checksum.
- Native signing/notarization remains an explicit future decision.

## Target tree

- [`windows/`](windows/)
- [`linux/`](linux/)
- [`macos/`](macos/)
