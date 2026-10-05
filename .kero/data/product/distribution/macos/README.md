# macOS distribution

macOS uses the same terminal-capable service, Qt visual client, and `kero.wasm`
core as the other supported platforms. The release container is DMG.

Artifacts receive a SHA-256 checksum and receive a detached GPG signature when
a key is configured. Apple code signing and notarization are currently
deferred.

## Why

The DMG is a native user-facing container while the executable semantics remain
shared with the portable core. The project does not present GPG verification as
a substitute for Apple notarization.
