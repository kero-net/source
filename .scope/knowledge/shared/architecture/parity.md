# Python-to-Rust Parity

| Capability | Python reference | Rust SCOPE | Required evidence |
|---|---:|---:|---|
| Independent result dimensions | complete | implemented | serialization tests |
| Canonical JSON and SHA-256 digests | complete | implemented | RFC 8785 and shared digest fixtures |
| HMAC authorization-artifact verification | complete | implemented | issue/verify and signed-DENY rejection tests |
| Layer 2 snapshot replay | complete | implemented | byte-equivalent replay; validation, decision-table, delegation, and restrictive-source tests |
| Exact workspace write | complete | pending | escape and crash tests |
| Bubblewrap command boundary | complete | pending | isolation tests |
| Git push and credential broker | complete | pending | local remote and secret tests |
| HTTPS service boundary | complete | pending | local TLS integration |
| Elevation lifecycle | complete | deferred from v1 | absence and deny-preservation tests |
| Recovery and rollback | complete | pending | fault-injection tests |

Parity is checked capability by capability. The Rust implementation does not
claim support until its row has direct local evidence.

## Version 1 release order

The project-local progress record is `.scope/checklists/release-v1.md`.
Implementation and evidence proceed in this order:

1. freeze the claimed version 1 scope and compatibility surface;
2. complete Layer 2 validation parity;
3. complete common Layer 3 audit, nonce, capability, and recovery primitives;
4. complete each approved Linux boundary with direct failure evidence;
5. harden the CLI and pass the full release-validation matrix;
6. prepare release identity, binaries, provenance, and publication rehearsal;
7. finish public documentation and repository/publication setup last, after
   supported behavior is frozen.

Documentation quality is a release gate, but public wording must not get ahead
of implemented and evidenced behavior.
