# Filesystem compatibility

KERO environment semantics do not depend on a particular host filesystem. A
repository or global home may be stored on any filesystem that safely provides
the portable baseline:

- regular files and directories;
- filename and relative-path identity;
- byte-for-byte file contents;
- basic timestamps; and
- basic read and write operations subject to host permissions.

Knowledge remains ordinary files. Its filename extension and content format do
not determine whether KERO can materialize it.

## Compatibility assessment

The host assesses the selected source and the destination filesystem when an
operation depends on a filesystem feature. Filesystem names such as NTFS,
exFAT, ext4, or NFS are useful diagnostics, not a capability contract: mount
options, operating-system versions, servers, and permissions can change the
actual behavior.

KERO preserves source relative paths exactly. It rejects a mount if a source
name cannot be represented unambiguously at the destination, including case
collisions and platform-reserved names. It never rewrites filenames, flattens
directories, or uses links, shortcuts, junctions, or a public/shared fallback
to make an incompatible source appear available.

Symbolic links, junctions, reparse points, and other non-regular entries are
outside the snapshot baseline and are rejected. This prevents a selected source
from becoming an unintended route to files outside its declared data root.

## Failure behavior

Snapshot materialization is fail-closed. Before reporting a mount as usable,
KERO must finish copying and validating the complete source tree, mark the
result read-only, and record runtime provenance. A failed operation removes its
partial destination and leaves existing local knowledge and existing mounts
unchanged.

Diagnostics identify the selected source, the affected relative path when
available, and the reason: unreadable data, unsupported entry type, incompatible
name, missing source, destination write failure, or unavailable host capability.
The user must repair the named condition or select a compatible location; KERO
does not silently substitute a weaker representation.

Mount inspection distinguishes a ready read-only snapshot from out-of-format
entries. A failed source assessment is reported to the requested mount operation
and is not evidence that its knowledge has become public or has been copied
elsewhere.

## Cross-filesystem operations

KERO never claims that an operation crossing filesystem or environment
boundaries is atomic. In particular, a move between local `data/` and a mounted
snapshot, or between two sources, is not a filesystem rename contract. A future
explicit copy operation may describe its own completion and recovery behavior,
but it must not present copy-and-delete as atomic rename.

## Decision

Read-only copied snapshots are KERO's portable mount mechanism. Live virtual
filesystem projections, including WinFsp- and FUSE-style designs, are not an
implementation fallback and are deferred to a separate transport and security
contract.

## Why

The snapshot boundary works without requiring a kernel or user-mode filesystem
provider and makes provenance, privacy, and failure handling explicit across
filesystems with different capabilities.
