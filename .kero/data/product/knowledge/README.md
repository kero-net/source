# Knowledge input

KERO captures knowledge only from an explicit user-selected file or directory.
It never inventories sibling directories, KERO home, mounted data, or arbitrary
machine paths as implicit input.

## Input snapshot contract

An accepted input is a regular file or a recursively traversed directory tree.
Traversal is deterministic: entries are ordered by their portable relative
path. Path names must be valid UTF-8. Symbolic links, device files, and other
non-regular entries are rejected.
A source contained beneath the selected repository's KERO data root is also
rejected, so importing cannot recursively capture its own output.

KERO stores each accepted input as an immutable local snapshot:

```text
.kero/data/input/<content-sha256>/
├── manifest.json
└── content/
```

The content digest covers the format version, entry kinds, portable relative
paths, and raw file bytes. `manifest.json` records the format version, digest,
and file count; it never records a host source path, credential, transport
locator, or other machine-specific fact. Re-importing identical content is
idempotent. Listing and removal verify that the manifest identifier, file
count, and stored content still agree; a corrupt snapshot is rejected rather
than treated as valid knowledge.

## Scope

Input snapshots are local knowledge. A mount remains an isolated external
knowledge scope and cannot be silently used as local input. Removing a snapshot
addresses only the explicitly named local input identifier.

## Why

Explicit selection and a byte-level identity make the captured input
reproducible without binding repository knowledge to the machine from which it
was imported. Rejecting links and unstable entry types avoids traversal escapes
and ambiguous content identities.

## Reconsider when

Archive formats, remote transports, mutable synchronization, semantic parsing,
and trust signatures each require their own explicit contracts. They must not
become implicit behavior of local snapshot capture.
