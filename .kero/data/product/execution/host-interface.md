# Core/host interface

**Status:** host ABI contract, version 1.

`kero.wasm` receives host effects through capabilities, never through
repository configuration or ambient machine paths. The companion
`wit/kero-host.wit` is the language-neutral boundary sketch; the Rust
`host::KeroHost` trait is its current testable representation.

## Filesystem scopes

Every filesystem request is a `scope + segments` value:

- `config` identifies the one environment `config` document;
- `local` resolves only under `data/`;
- `mount(name)` resolves only under `mnt/<name>/`;
- `runtime` resolves only under `.runtime/`;
- `mounts` is an initialization-only capability for the empty mount root.

Segments cannot include separators, `.` or `..`.

Mounted scopes are read-only data capabilities. A host may read a validated
materialized mount but must deny write and directory-creation requests beneath
`mount(name)`. Removing or refreshing a mount is a separate mount-management
operation, never a write through the mounted knowledge scope.

The host retains control of filesystem APIs, process execution, networking,
mount transfer, credentials, hidden attributes, and diagnostic presentation.

## Lifecycle capabilities

The lifecycle capability lets a host report an existing or nearest boundary,
apply its hidden-file convention to runtime state, and remove runtime state.
Core initialization is idempotent: it writes only the comment-only `config` and
creates `data/` and `mnt/`. It does not create `.runtime/`.

## Why

Scopes describe what an operation is allowed to touch without exposing an
absolute machine path to the portable core. This makes path traversal and
local/mounted-data confusion impossible to express through the normal
interface rather than merely forbidden by convention.

## Consequences

The core exposes no CPU, OS, native target, absolute-path, or installer policy.
Windows, Linux, and future hosts package the same WASM file and implement these
effects independently.
