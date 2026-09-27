# Portable core

`kero.wasm` is the one portable KERO core artifact. It carries the semantic
model shared by every supported host target.

The core has no ambient access to machine paths, networking, credentials,
process execution, installer state, CPU identity, or operating-system identity.
Effects are supplied explicitly through the host interface.

Before an operation, a host loads the WASM artifact with no ambient imports,
calls `kero_host_abi_version`, and requires ABI version 1. A missing artifact,
invalid module, or ABI mismatch is a host runtime error.

## Why

Keeping one artifact as the semantic authority makes parity testable. A native
host cannot silently reinterpret repository behavior because a platform API is
more convenient.

## Reconsider when

A future core format may replace WASM only if it preserves the same portable,
capability-scoped semantic boundary. Adding a native fallback implementation is
not equivalent.
