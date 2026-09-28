# Mount synchronization

## Decision

New mounts default to manual, read-only snapshots. A repository may opt a named
mount into event refresh. The local service watches the explicitly recorded
source data root, debounces events, validates a complete replacement snapshot,
and publishes it only after success. There is no default periodic full-tree
audit; startup, resume, overflow, status/access, and manual refresh revalidate
the source.

Writable synchronization is restricted to same-user directly accessible local
sources. It requires a signed source-owned KST grant that binds a collaborator
identity, mount, direction, baseline, and expiry. Source-only and local-only
changes synchronize in their granted direction. Divergent edits block as a
conflict until the user chooses source, local, or export-both.

The collaborator identity is an existing per-user Ed25519 identity. KERO only
creates it during the explicit `setup --identity reuse` action; grant and sync
commands refuse to create key material as a side effect.

## Runtime records

Generated mount provenance is KST at `.runtime/mounts/<name>.kst`. It records
the explicit source location, digest, source format, file count, access,
refresh mode, baseline, and status. It is disposable, host-local state and is
never repository configuration. Unreleased JSON provenance is repaired by
revalidating its recorded source and rewriting KST while preserving the prior
snapshot if repair fails.

## Why

Native filesystem events avoid recurring scans while a staged snapshot retains
portable validation and read-only safety. A source grant, rather than local file
permissions, makes writable synchronization explicit and revocable. Blocking
divergence avoids silently discarding knowledge.

## Consequences

Watchers are event-driven helpers, not live projections. They can miss events,
so KERO validates on lifecycle and user-visible triggers. Remote and cross-user
transport require a separate security and service-transport decision.
