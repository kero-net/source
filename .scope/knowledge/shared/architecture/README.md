# Architecture

SCOPE has three non-collapsible layers:

1. Knowledge resolution locates applicable human-readable contracts.
2. Pure policy resolution returns `ALLOW` or `DENY` with immutable provenance.
3. Enforcement verifies the artifact, binds it to a native operation, observes
   capability, and mediates execution within an explicit threat model.

The independent result dimensions are:

```text
authorization = ALLOW | DENY
verification  = VALID | INVALID | STALE | UNVERIFIABLE
capability    = CAN | CANNOT | UNKNOWN
enforcement   = ADVISORY | ENFORCED
execution     = EXECUTE | BLOCK
```

The initial implementation is Linux-specific at the native boundary. Policy
and serialized contracts remain platform-neutral so another native adapter can
be added later without changing Layer 2 semantics.

The Python reference implementation remains outside this repository during the
port. Its passing tests and immutable fixtures are an oracle, not a runtime
dependency.

## Canonical JSON

Cryptographic digests and signatures bind to RFC 8785 JCS bytes.
Canonicalization must preserve JSON values exactly. String contents are opaque
and unrestricted by this layer; it does not normalize, classify, filter, or
reinterpret text.
