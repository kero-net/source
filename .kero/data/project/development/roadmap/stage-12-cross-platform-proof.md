# Stage 12 — Cross-platform proof

**Planned.** Build and validate the refined product on each supported native
platform and architecture.

Canonical targets, toolchain rules, and package evidence remain under
[`product/distribution`](../../../product/distribution/). Local cross-build or
emulated evidence is useful diagnostic evidence but does not replace native
runtime proof. macOS ARM64 remains coming soon until a Mac-native contributor
provides the required toolchain and evidence.

## Completion evidence

- Every enabled target has a static package validation record.
- Every enabled release target has the required native runtime evidence.
- Package architecture, payload dependencies, checksums, and startup behavior
  are consistent for each native target.

Stage 13 uses these validated artifacts for final release checks and
publication.
