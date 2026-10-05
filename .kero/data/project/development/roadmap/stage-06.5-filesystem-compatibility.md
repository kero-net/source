# Stage 06.5 — Filesystem compatibility

**Complete for the portable local snapshot contract.** The mount materializer
now preserves its privacy and path-identity guarantees across compatible host
filesystems.

Implements: [`filesystem compatibility`](../../../product/environment/filesystem-compatibility.md)
and the portable snapshot rules in [`mounts`](../../../product/environment/mounts/).

## First pass: implementation inventory

The current materializer already copies only `data/`, rejects symbolic links and
unsupported entries, removes a partial destination after a copy failure, records
runtime provenance, and marks a completed snapshot read-only. The remaining
work is intentionally narrow and belongs in the repository mount materializer:

- validate every source path component against the destination filesystem before
  materialization can report success, including destination-specific reserved
  names and case collisions;
- preserve portable basic timestamps when copying regular files and directories;
- make failed-copy diagnostics name the source-relative path and failure class;
- stage output privately and publish it only after validation, copy, timestamp,
  provenance, and read-only setup have completed; and
- add host-independent contract tests plus platform-specific tests where link
  or filename behavior requires the native platform.

No live WinFsp/FUSE projection, filename rewrite, link substitution, or
public/shared fallback is part of this stage. The former blanket no-writable-
mount decision is superseded by the scoped signed synchronization contract in
[`mount synchronization`](../../../product/environment/mounts/synchronization.md);
the snapshot safety rules here remain mandatory for every publication.

## Current evidence

- The materializer copies ordinary files with arbitrary extensions, preserves
  modified timestamps, and continues to reject symbolic links and unsupported
  entries.
- A snapshot is assembled below `.runtime/mount-staging/`; it becomes visible
  under `mnt/` only after its copy, timestamps, provenance, and read-only state
  are complete.
- Destination case behavior is probed rather than inferred from a filesystem
  name. Case collisions on a case-insensitive destination and Windows reserved
  or invalid names are rejected.
- `cargo test --manifest-path src/Cargo.toml -p kero-core` passes on the
  Windows host. Unix link-rejection coverage remains native-platform coverage.

## Completion evidence

- Snapshot materialization accepts ordinary files with arbitrary extensions and
  preserves their relative paths, bytes, and portable timestamps.
- Incompatible names, case collisions, links, unsupported entries, unreadable
  content, and destination write failures leave no visible partial mount and
  return an actionable source-relative diagnostic.
- Existing snapshots and repository-local `data/` remain unchanged when a new
  mount fails.
- The core test suite covers portable behavior; native filesystem evidence is
  recorded under the cross-platform proof stage rather than relabelled as a
  generic core test.
