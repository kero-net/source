# Version 1 Release Scope

This document freezes the supported product scope for SCOPE's first public
version 1 release. It narrows release claims without changing the canonical
Layer 1, Layer 2, or Layer 3 contracts.

## Supported platform

Version 1 is Linux-first. The Rust policy model and serialized contracts remain
platform-neutral, but native execution support is Linux-only. Windows, macOS,
mobile platforms, containers as a separate trust boundary, and Flatpak
packaging are outside the version 1 support claim.

## Required capabilities

Version 1 includes:

- global and project `.scope/` environment discovery and initialization;
- registered human-readable knowledge resolution surfaces;
- pure Layer 2 policy loading, validation, authorization, explanation,
  immutable snapshots, and snapshot replay;
- canonical JSON, SHA-256 digests, and authenticated single-use authorization
  artifacts;
- common Layer 3 verification, nonce, audit, capability, safe-disable,
  generation, deployment-attestation, and recovery infrastructure;
- exact Linux `filesystem.write` through `broker.workspace-write/v1`;
- no-network `shell.execute` through `sandbox.command/bwrap-v1`;
- exact-refspec `git.push` with optional brokered credentials;
- exact-destination HTTPS `service.invoke`;
- structured independent authorization, verification, capability,
  enforcement, execution, and reason results.

Every capability above requires the direct evidence named by the project parity
matrix and `.scope/checklists/release-v1.md` before it may be documented as
supported.

## Deliberately unsupported

Version 1 does not provide:

- elevation, emergency bypass, root-role shortcuts, interactive sudo, or deny
  bypass;
- arbitrary shell execution, interactive TTYs, writable sandbox host paths,
  or sandbox network access;
- directory creation, deletion, chmod, links, multi-target writes, or broad
  filesystem mediation;
- force pushes, deletion or wildcard refspecs, caller-controlled Git hooks,
  ambient credential helpers, or interactive Git prompting;
- HTTP, redirects, proxies, raw sockets, streaming, or multiple service
  destinations;
- a global `ENFORCED` claim or automatic fallback from enforced to advisory;
- compatibility guarantees for undocumented internal Rust APIs.

Unsupported operations fail closed or report a stable unsupported/capability
result without changing Layer 2 authorization history.

## Compatibility surface

The version 1 compatibility surface consists of:

- documented CLI command names, options, JSON result schemas, and exit
  behavior;
- accepted TOML environment, record, and request schemas;
- snapshot and authorization-artifact schemas plus canonical bytes and digest
  behavior;
- resolver, typed-scope implementation, boundary, adapter, threat-model,
  audit, checkpoint, and deployment-attestation identifiers;
- documented `.scope/` consumer layout and ignored machine-local state.

Any incompatible change to that surface requires a new schema, resolver,
boundary, adapter, or major product version as appropriate. Human-readable
messages and undocumented internal library organization may change while
stable reason codes and serialized meanings remain compatible.

## Enforcement claim

`ADVISORY` is a supported version 1 result. `ENFORCED` is returned only when a
current authenticated deployment attestation proves the exact boundary,
threat model, protected paths, mechanisms, and absence of in-scope alternate
writers or executors. Without that evidence, an otherwise valid operation
remains `ADVISORY`; it is never silently upgraded or downgraded.

## Release gate

Public documentation and repository presentation are completed only after the
required capabilities and compatibility surface pass their direct evidence.
The release must identify unsupported behavior plainly and must not imply that
local wrappers provide host-global enforcement.
