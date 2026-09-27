# Distribution

KERO release artifacts are built from the authoritative source and distributed
per native host target. Platform packaging may differ, but it never changes
KERO core semantics or creates a platform-specific product version.

## Integrity policy

Every package receives a SHA-256 checksum. Contributor packages are unsigned
by default and never require credentials or key material. A detached signature
is added only by a maintainer-controlled release environment that explicitly
enables signing. The KERO public GPG key and its full fingerprint accompany
releases that use those signatures.

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

Optional Hyper-V and QEMU adapters are local contributor configuration, not
dependencies. macOS ARM64 is coming soon until a Mac contributor supplies the
Apple toolchain and native evidence.

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
