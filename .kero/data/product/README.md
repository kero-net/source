# KERO product

This branch contains durable product truth: KERO's environment model, portable
execution model, host behavior, knowledge-input, processing/trust, and
distribution contracts.

KERO currently establishes these boundaries before higher-level knowledge
processing and trust semantics:

1. local knowledge and mounted knowledge remain distinct;
2. repository configuration remains machine-independent;
3. one portable WASM core owns KERO semantics;
4. a native service provides scoped platform capabilities and terminal access;
5. installation and UI remain service clients rather than alternate core
   implementations.

Indexing/query behavior, serialization/compression, semantic transformations,
remote trust roots, and production mount transport are intentionally later
product contracts. Their future design
may build on these boundaries but should not casually redefine them.

The implemented local capture contract is defined in the knowledge branch. It
is intentionally narrower than semantic parsing. Deterministic local
processing and detached trust signatures are defined in the processing branch.

## Why

These boundaries make platform support and later processing features additive.
A new operating system, CPU architecture, UI host, or transport should not
change what existing KERO data means.
