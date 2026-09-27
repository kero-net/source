# Execution model

KERO has one portable semantic core, a headless native service, and optional
clients:

```text
KERO source
    ↓
kero.wasm
    ↓
native KERO service
    ├── terminal interface
    ├── Qt visual client
    └── platform installation/package adapters
```

The core owns KERO semantics: environment layout, configuration semantics,
scope separation, provenance rules, and deterministic transformations. The
native service hosts that core and owns platform effects such as filesystem
access, process execution, networking, sandboxing, platform paths, privilege
checks, hidden attributes, and identity/key-store integration. Terminal and
Qt surfaces request service operations; neither becomes a second semantic core.

## Decision

CPU architecture and operating system are host deployment facts. Core behavior
must not branch by target triple, CPU family, or operating system to change
KERO semantics.

KERO functionality is exposed through a terminal-capable service before a GUI
depends on it. The GUI is a visual client of that service, not its only entry
point or its owner of repository, mount, knowledge, processing, or trust logic.

## Why

A single portable core prevents each platform host from becoming its own KERO
implementation. Platform support can then change independently from the meaning
of data and operations.

## Consequences

The intended core distribution is `kero.wasm`. The native service validates and
hosts that artifact rather than falling back to a separate native core. A GUI
may add usability, but the supported operations remain usable without it.

The capability boundary between the two sides is defined in
[`host-interface.md`](host-interface.md). The service role is defined in
[`service.md`](service.md).
