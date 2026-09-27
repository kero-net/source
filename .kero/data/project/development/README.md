# Development

This branch contains source-workflow and implementation-progress knowledge. It
may change as tooling changes without redefining KERO product semantics.

## Current local distribution entry points

The source-owned wrapper builds a local development package for the current
host target:

```text
kero.cmd build current    # Windows
./kero build current      # Linux or macOS
```

The wrappers invoke the Lua distribution runner internally. A build creates a
package, checksum, and static validation record beneath `.heap/`; it does not
need GitHub CLI, credentials, a clean Git tree, or a signing key. Runtime
testing is separate (`kero.cmd test current --adapter native` or
`./kero test current --adapter native`) because a cross-build cannot prove a
native loader or runtime dependency.

`act` is an optional portable/Linux-container check. It never represents a
Windows or macOS virtual-machine test. Native desktop testing runs on the
matching local host or an explicitly configured local adapter. Canonical
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

This document replaces the prior remote-CI parity description. Normal package
creation is local and unsigned by default; remote source review does not build
or sign contributor artifacts. The separate build/test evidence contract avoids
mistaking a cross-build or emulated run for native release proof.

Implementation progress is tracked under [`roadmap/`](roadmap/). The
prototype-to-Qt migration state is recorded in
[`qt-host-transition.md`](qt-host-transition.md). The active shared-editor and
build-tooling work is recorded in
[`contributor-tooling.md`](contributor-tooling.md). The separate non-delivery
representation-machine research track is recorded in
[`representation-machine-research.md`](representation-machine-research.md).
