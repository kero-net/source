# Runtime state

`.runtime/` is hidden, disposable host-owned state. It may contain locks,
caches, transport data, or other ephemeral execution material, but it is never
a source of truth for KERO knowledge. Local-service state is generated KST at
`.runtime/service.kst`; it identifies a current-user-only named pipe on Windows
or owner-only Unix-domain socket on Unix hosts, never a network port.

Generated mount provenance is KST beneath `.runtime/mounts/`; absolute source
paths belong there because they must never become portable repository policy.
`.runtime/mount-staging/` is private workspace for validating a complete mount
replacement before publication beneath `mnt/`.

Initialization does not create `.runtime/`. An operation calls
`ensure_runtime` only when disposable state is required. `reset_runtime`
removes only the runtime scope and cannot remove `config`, local data, or
mounted data. Recreation is explicit through a later `ensure_runtime` call.

## Why

Separating runtime state from durable knowledge makes cleanup safe and prevents
implementation artifacts from being mistaken for authored or mounted content.
