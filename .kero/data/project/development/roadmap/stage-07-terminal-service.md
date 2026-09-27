# Stage 07 — Terminal service

**In progress.** Expose completed repository, mount, knowledge, processing, and trust
operations through the native service and a first-class terminal interface.

The service is the headless product entry point. It hosts the validated WASM
core, enforces scoped host capabilities, and remains usable without a GUI. It
persists locally per signed-in user and is restored by the installation logon
task before a terminal or GUI client needs it. Its local IPC contract uses
named pipes on Windows and Unix-domain sockets on Unix hosts; it does not open
a network listener.

## Completion evidence

- The CLI can discover, start, and communicate with the local service without
  opening a TCP or UDP listener.
- Repository, mount, knowledge, processing, and trust operations have stable
  terminal commands and useful errors.
- The service lifecycle is deterministic: start, reconnect, stop, and recovery
  do not require the Qt client.

Stage 08 may begin after this service boundary is usable end to end.

## Current evidence

The terminal host now classifies explicitly selected sources as repository,
global-home, or out-of-format; reports malformed mount entries instead of
hiding them; and materializes repository or explicit global-home data as a
read-only mount with runtime provenance. Detached Ed25519 signatures provide
artifact integrity verification; encryption is not implemented.

The persistent service, authenticated request path, lifecycle controls, and
ordinary CLI routing are implemented. Completion packaging evidence requires a
release bundle containing `kero-host.exe` beside `kero.wasm`, installer payload
validation, persisted `KERO_HOME`, PATH exposure, and Windows install smoke
verification. Qt mount controls remain deferred to the following GUI stage.

Mount synchronization now extends this service boundary: the service owns
event-driven refresh watchers and stages a validated replacement snapshot before
publication. The narrow Mount Sync dialog is service consumption for refresh
and conflict actions; it does not implement the deferred general Mounts page.
