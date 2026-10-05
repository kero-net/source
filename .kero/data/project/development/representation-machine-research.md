# Representation-machine research

## Status

This is an active non-delivery research track. It records the work and evidence
needed before representation-machine behavior can become a numbered product
implementation stage.

The canonical product direction is
[`product/processing/reversible-representation`](../../product/processing/reversible-representation.md).
This note does not create a CLI command, storage format, public API, or release
commitment.

## Research sequence

1. Prove byte-exact reconstruction through a bounded, deterministic runtime
   interpreter.
2. Add an established codec primitive and measure raw, DEFLATE, and Zstandard
   baselines.
3. Measure corpus-wide content references and shared representation material.
4. Measure delta reconstruction for related regions and files.
5. Compare mixed compositions of primitives across corpus regions.
6. Start with bounded exhaustive selection, then investigate selection
   heuristics or search only when measurements justify them.
7. Investigate reversible structure-aware transforms for source and structured
   data after the general machinery works.
8. Investigate structural and semantic guarantees separately from exact mode.

## Evidence requirements

The first prototype uses a checked-in, license-reviewed corpus containing text,
source code, and structured data. Every candidate must prove byte-for-byte
round-trip reconstruction and deterministic repeatability.

Reports compare raw data, DEFLATE, Zstandard, individual-file compression, and
whole-corpus compression where applicable. They record program bytes, payload
bytes, total representation bytes, encode time, decode time, and tracked peak
heap use. Results preserve failures and unsupported cases, and present a Pareto
frontier rather than an arbitrary combined score. Random access is explicitly
unsupported until a later experiment defines an access primitive and measure.

## Prototype boundary

The initial prototype is isolated from `kero-core`, the service, Qt, persistent
`.kero` state, and release packaging. Its serialization and experimental
contracts are disposable. Its implementation is not presumed disposable:
components can be promoted when benchmark evidence supports a stable product
contract.

This research track does not renumber Stage 08. The existing Stage 08 remains
the Qt visual client; future evidence may create new numbered implementation
stages after the research direction is mature.
