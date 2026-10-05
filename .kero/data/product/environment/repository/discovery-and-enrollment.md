# Repository discovery and enrollment

**Status:** implemented local repository contract. Remote mount transport and
its authorization are separate later host contracts.

## Input boundary

The host receives one explicit starting directory or command path. It resolves
that path, asks the Git host capability whether it belongs to a non-bare
worktree, and searches only ancestors of that path for the nearest `.kero`
boundary.

KERO must not search sibling directories, drive roots, `$HOME`, or `KERO_HOME`
for repositories. A parent KERO boundary matters only when it lies on the
selected path's ancestry and is therefore the nearest boundary.

## Discovery result

| State | Meaning | Mutation allowed by automatic policy |
|---|---|---|
| `notRepository` | The selected path is not in a Git worktree. | No |
| `unsupported` | The Git target is bare or lacks a usable worktree. | No |
| `unavailable` | Required metadata or write access cannot be obtained. | No |
| `enrolled` | The nearest applicable boundary has `.kero/config`. | No; report it |
| `eligible` | A writable non-bare worktree has no nearer KERO boundary. | Yes |

Git revision is repository context only. If a later pass consumes a modified
working-tree file, its content hash rather than `HEAD` binds the bytes actually
used.

## Enrollment policies

`ask` produces an enrollment proposal with the selected worktree, target
boundary, and planned effects. It changes nothing until the caller approves.

`automatic` initializes only an `eligible` explicitly selected worktree. It
creates `.kero/config`, `.kero/data/`, and `.kero/mnt/`, then asks Git for the
resolved local exclude path before adding KERO runtime exclusions. Re-running
the operation is idempotent.

`manual` never initializes implicitly. It may describe the command or action
the user must take, but makes no repository or Git metadata change.

`kero init [path]` is the explicit initialization command for an eligible
non-bare worktree. It rejects a path outside Git rather than creating a
repository-style `.kero/` boundary in an arbitrary directory. Global KERO home
creation remains a separate setup operation.

## Why

Discovery classifies one user-selected target rather than discovering machines.
Enrollment policy then controls whether that known target may be mutated. This
keeps selection, authorization, and mutation separate.

## Mounting handoff

A mount operation may use the discovery result only to establish its target
environment. It writes only beneath that environment's `mnt/<name>/` and cannot
infer a source from repository configuration.

## Acceptance fixtures

- selected nested path in an eligible worktree;
- selected nested path below an already enrolled boundary;
- selected directory outside Git;
- bare or otherwise unsupported repository;
- unwritable worktree or Git metadata;
- linked worktree whose resolved exclude file is not root `.git/info/exclude`;
- ask/manual no-mutation assertions;
- automatic idempotence and one-target-only assertions.

The native fixture suite currently covers nested eligible discovery, an
existing boundary, ask/manual no-mutation, automatic enrollment, a path outside
Git, bare repositories, and linked-worktree exclude resolution. Unwritable
worktrees remain required coverage.
