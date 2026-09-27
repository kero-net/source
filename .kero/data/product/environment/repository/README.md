# Repository environment

A repository KERO environment is an explicit `.kero/` boundary inside a
non-bare Git worktree. KERO operates on one selected repository target at a
time; it does not build a background inventory of the machine.

The repository owns its own `config`, local `data/`, and explicit mounts. Home
configuration may provide defaults to the host but does not become repository
input.

Repository discovery and enrollment are defined in
[`discovery-and-enrollment.md`](discovery-and-enrollment.md).

## Why

An explicit target keeps repository effects predictable. Searching sibling
folders, drive roots, or user homes would turn a local action into an ambient
machine-wide discovery mechanism and make authorization unclear.
