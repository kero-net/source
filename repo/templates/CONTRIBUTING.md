# Contributing to KERO

KERO is currently a pre-1.0 knowledge-system project. Contributions should
follow the accepted semantic and compiler contracts and must not reintroduce
discarded authorization, permission, or execution-enforcement architecture.

Canonical development happens in `kero-net/source`; the public channel branches
in `kero-net/kero` are generated.

Run the Rust checks from a source checkout:

```bash
cargo fmt --manifest-path code/Cargo.toml --all -- --check
cargo clippy --manifest-path code/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path code/Cargo.toml --all-targets --all-features --locked
```

Changes to semantic identity, provenance, canonicalization, or encoding require
updated contracts and fixtures in the same change. Keep CLI behavior as an
adapter over `kero-core` rather than a second product model.
