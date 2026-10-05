# Stage 05 — Knowledge input

**Complete for the local snapshot contract.** Broader input acceptance and
provenance coverage require explicit contract extensions; remote, archive,
semantic parsing, and trust behavior remain separate work.

This stage establishes the service functionality consumed by both terminal and
visual clients. The initial local snapshot contract is canonical under
[`product/knowledge`](../../../product/knowledge/); archive, remote, and
semantic-input behavior remain out of scope until separately specified.

## Current evidence

The terminal host supports `kero knowledge add|list|remove`. Adding an
explicit regular file or directory creates a deterministic, content-addressed
snapshot beneath `.kero/data/input/`, rejects links and KERO-owned local or
mounted input, records no host source path, and is idempotent for identical
content. The built CLI/WASM smoke test exercises add, list, and remove through
the command executable; CI installs the WASM target and runs that test on
Windows, Linux, and macOS.
