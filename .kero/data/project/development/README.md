# Development

This branch contains source-workflow and implementation-progress knowledge. It
may change as tooling changes without redefining KERO product semantics.

## Current local distribution entry point

VS Code exposes one task: **Kero: Validate Locally**. It first runs the
credential-free Linux GitHub workflow jobs through `act` from an isolated
snapshot of the current source tree, then builds every enabled distribution
through the source-owned native runner. No target or stage is skipped.

`act` is required for Linux workflow validation. It does not represent a
Windows or macOS virtual-machine test. Native desktop testing runs on the
matching local host or supported emulation. Canonical
target, toolchain, evidence, and release rules live under
[`../../product/distribution/`](../../product/distribution/).

## Release direction

Contributors open source-review pull requests and may attach a candidate
manifest after local validation. Local output remains ignored under `.heap/`.
Maintainers reproduce required native evidence and sign approved artifacts from
their protected local environment. Publication is deliberately deferred to
roadmap stage 13, after application/CLI integration, functional testing,
interface refinement, and cross-platform proof.

## Replacement record

This document replaces the optional `act` decision: local validation now runs
the Linux GitHub workflow jobs before native package assembly. Normal package
creation is local and unsigned by default. A Linux container or cross-build
still cannot count as native Windows or macOS runtime evidence.

Source-review CI validates repository contracts, Rust, automation, documentation,
localization, and generated Pages/repository inputs. It does not build platform
packages. After a successful CI gate on a `source` push, the publication
decision may call the separate publication workflow; that workflow builds
packages only when the authored publication record enables release. This
replaces the earlier CI package matrix that ran before publication was decided.

Implementation progress is tracked under [`roadmap/`](roadmap/). The
prototype-to-Qt migration state is recorded in
[`qt-host-transition.md`](qt-host-transition.md). The active shared-editor and
build-tooling work is recorded in
[`contributor-tooling.md`](contributor-tooling.md). The separate non-delivery
representation-machine research track is recorded in
[`representation-machine-research.md`](representation-machine-research.md).
