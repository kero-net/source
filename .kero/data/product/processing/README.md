# Processing and trust

KERO processing derives a deterministic artifact from one explicit, validated
local knowledge-input snapshot. Trust signs an explicit artifact file; neither
operation reads a key, source path, or transport locator from repository
configuration.

## Processing artifact contract

The command "kero process build <input-id>" reconstructs this generated artifact
beneath ".kero/.runtime/processed/<input-id>.json":

    {
      "format_version": 1,
      "input_id": "<content-sha256>",
      "files": [
        { "path": "guide.md", "sha256": "<content-sha256>", "bytes": 123 }
      ]
    }

Entries are ordered by portable UTF-8 path. Each file digest covers its raw
bytes. The artifact has no clock, host path, source locator, credential, or
machine identity, so the same input snapshot reconstructs identical bytes.
The command "kero process verify <input-id>" reconstructs the expected artifact
and compares it with the generated artifact in the runtime path.

The artifact is disposable derived state, not new local knowledge. Rebuilding
replaces only its corresponding runtime artifact.

## Trust contract

The command "kero trust sign <artifact> --key <key-file>" produces a detached
JSON signature beside the artifact unless "--output" selects another explicit
path. The key file contains exactly one 32-byte Ed25519 seed encoded as 64
lower-case hexadecimal characters. KERO never copies this key into a
repository, KERO home, runtime artifact, or signature.

A signature records format version, algorithm, artifact SHA-256, Ed25519 public
key, and signature bytes. "kero trust verify <artifact>" validates both the
artifact digest and detached signature. Verification needs only the artifact
and signature; it does not need the private key.

This is integrity and origin verification, not encryption. KERO currently has
no encryption-at-rest, encrypted mount transport, key escrow, or automatic key
discovery contract. Those features must define their key ownership, recovery,
and access boundaries before implementation.

## Why

Deterministic processing makes reconstruction independently checkable before
semantic transformations are added. Detached signatures bind exactly the
artifact bytes that a user chooses to distribute without turning repository
configuration into a key store.

## Deferred

Semantic parsing, indexes, archive inputs, remote trust roots, key discovery,
revocation, threshold signatures, timestamping, and publication policy require
separate contracts. Ed25519 detached signatures in this stage do not claim
platform code-signing or notarization.

Reversible corpus representation is an active research direction under
[`reversible-representation.md`](reversible-representation.md). It does not
alter this local processing/trust contract or establish a permanent storage
format.
