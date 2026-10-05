# Stage 04 — Repository and mounts

**Complete for the local mount contract.** Explicit repository
discovery/enrollment and named local materialized mounts are implemented.
Remote transport, synchronization, and authentication remain deferred until a
separate host transport contract is accepted.

Implements: [`repository discovery and enrollment`](../../../product/environment/repository/discovery-and-enrollment.md) and [`mounts`](../../../product/environment/mounts/).

## Current evidence

The terminal host supports explicit repository status/enrollment and
`kero mount create|add|list|remove`. Local materialization copies only a
selected source environment's `data/` into the target's named mount, records
runtime-only provenance, rejects self-mounting and links, and never stores a
source path in repository configuration.
