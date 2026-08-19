# Python-to-Rust Parity

| Capability | Python reference | Rust SCOPE | Required evidence |
|---|---:|---:|---|
| Independent result dimensions | complete | implemented | serialization tests |
| Canonical JSON and SHA-256 digests | complete | implemented | RFC 8785 and shared digest fixtures |
| HMAC authorization-artifact verification | complete | implemented | issue/verify and signed-DENY rejection tests |
| Layer 2 snapshot replay | complete | in progress | byte-equivalent result; remaining validation parity |
| Exact workspace write | complete | pending | escape and crash tests |
| Bubblewrap command boundary | complete | pending | isolation tests |
| Git push and credential broker | complete | pending | local remote and secret tests |
| HTTPS service boundary | complete | pending | local TLS integration |
| Elevation lifecycle | complete | pending | profile and revocation tests |
| Recovery and rollback | complete | pending | fault-injection tests |

Parity is checked capability by capability. The Rust implementation does not
claim support until its row has direct local evidence.
