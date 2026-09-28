# Native service host

A native service host is the platform adapter around `kero.wasm`. It owns
native filesystem, Git, installation, environment-variable, process, and
package conventions. It exposes KERO operations to a terminal interface and
to optional visual clients. It is not a platform-specific KERO product or a
second core.

## Runtime admission

Before offering a KERO operation, the host validates the portable core through
the ABI handshake defined by [`../host-interface.md`](../host-interface.md).
It never falls back to an unvalidated native semantic implementation.

The host locates its sibling `kero.wasm` by default. Embedders and development
invocations may explicitly supply `--runtime <path>` or set `KERO_RUNTIME`;
neither mechanism belongs in repository configuration.

The current ABI establishes artifact compatibility. A later ABI revision may
move command dispatch and more capability calls fully into the WASM core; until
then, already-defined scoped operations may still be delegated through native
adapters after validation.

## Service operations

The service exposes explicit repository status, repository enrollment, mount
add, and mount remove operations. All repository targets and mount sources
remain explicit. The service does not scan KERO home or disks for repositories.

The exact executable, service-lifecycle, and client protocol surface is
implementation state rather than a core semantic contract. It must preserve
the explicit-operation and scoped-capability rules in this branch.

## Why

Platform APIs are necessary, but platform semantics are not. Keeping the host
small and effect-oriented lets native integration evolve without forking the
product model.

## Visual client

KERO's current visual client and installation implementation uses Qt 6. It is
a client of the service rather than the owner of KERO functionality. See
[`qt/`](qt/).
