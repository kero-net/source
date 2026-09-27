# Linux distribution

Linux uses the same terminal-capable service, Qt visual client, and `kero.wasm`
core as the other supported platforms. The release container is AppImage.

Published Linux artifacts require both a SHA-256 checksum and a detached GPG
signature.

## Why

AppImage provides a self-contained desktop package while preserving the common
host/core model. Requiring the configured GPG signature before publication
provides the project's current explicit trust signal where no platform-native
signing program is being used.
