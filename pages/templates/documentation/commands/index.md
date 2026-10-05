# KERO command reference

The command operates on explicit paths. Run `kero-host --help` or
`kero-host <command> --help` for argument details.

## Repository and setup

```text
kero-host init [path]
kero-host repository status [path]
kero-host repository enroll --policy ask|automatic|manual [path]
kero-host setup [--home user|system|none|<absolute-path>] [--enrollment ask|automatic|manual] [--identity reuse|configure-later]
```

`init` accepts only an eligible non-bare Git worktree. `setup` creates a marked
global KERO home. `setup --identity reuse` explicitly creates or reuses the
per-user Ed25519 identity needed for source grants and writable sync; those
commands never generate an identity implicitly.

## Mounts

```text
kero-host mount create <name> [--repository <path>]
kero-host mount add <name> <source> [--repository <path>]
kero-host mount add-home <name> [--home <path>] [--repository <path>]
kero-host mount list [--repository <path>]
kero-host mount remove <name> [--repository <path>]
kero-host mount refresh <name> [--repository <path>]
kero-host mount repair <name> [--repository <path>]
kero-host mount watch enable|disable <name> [--repository <path>]
kero-host mount grant create <name> --target-key <public-key> --direction pull|push|bidirectional --source <path> --expires <unix-seconds> [--repository <path>]
kero-host mount grant list --source <path>
kero-host mount grant revoke <name> --target-key <public-key> --source <path>
kero-host mount sync <name> [--repository <path>]
kero-host mount conflict use-source|use-local <name> [--repository <path>]
kero-host mount conflict export <name> --output <outside-mount-path> [--repository <path>]
```

Mount sources are explicit KERO environments. A mount publishes a validated
isolated snapshot. Watch mode is event-driven and debounced, not a live
filesystem projection. Writable synchronization is local, same-user only and
requires a source-owned signed grant; divergent source and mounted edits block
until the user selects source, local, or a provenance-labelled export of both.
`mount repair` converts unreleased JSON provenance by revalidating the recorded
source and publishing KST provenance; a failed repair leaves the prior complete
snapshot visible.

## Knowledge input

```text
kero-host knowledge add <file-or-directory> [--repository <path>]
kero-host knowledge list [--repository <path>]
kero-host knowledge remove <input-id> [--repository <path>]
```

Input creates a deterministic local snapshot. Links and KERO-owned local or
mounted input are rejected.

## Processing and trust

```text
kero-host process build <input-id> [--repository <path>]
kero-host process verify <input-id> [--repository <path>]
kero-host trust sign <artifact> --key <seed-file> [--output <signature-file>]
kero-host trust verify <artifact> [--signature <signature-file>]
```

Process build reconstructs a deterministic file-digest artifact in repository
runtime state. Trust sign uses an explicit Ed25519 seed file containing 64
lower-case hexadecimal characters. Trust verify validates the artifact digest
and its detached signature without the private key.

Use `--json` before any command to receive machine-readable results. Use
`--runtime <kero.wasm>` only for development or embedding.
