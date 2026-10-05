# Mounts

A mount exposes knowledge from one explicit KERO environment beneath the target
repository's `.kero/mnt/<name>/` root. Mount names are stable lower-case
identifiers; a source is never discovered from ambient machine state.

Mounts begin as read-only snapshots. Their source data is copied, validated,
and recorded in disposable runtime provenance. Local `data/` never falls
through into a mount and mounted data cannot overwrite local knowledge.

The mount source must be an explicit in-format repository or global home. KERO
copies only its `data/` tree, rejects links, unsafe names, inaccessible content,
and incompatible destination names, and preserves portable timestamps. A mount
never recursively copies another mount or runtime state.

`mount-staging` is a private publication workspace beneath `.runtime/`. KERO
builds and validates a complete replacement there, then publishes it beneath
`mnt/`; it is neither a second mount nor a source of truth. A failed publish
removes staging and preserves the prior visible snapshot.

Continuous refresh and grant-authorized writable synchronization are defined in
[`synchronization.md`](synchronization.md). They do not create a live filesystem
link or permit unapproved writes to a source.
