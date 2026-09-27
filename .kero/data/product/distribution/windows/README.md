# Windows distribution

Windows packages the terminal-capable KERO service, Qt visual client, and the
same portable `kero.wasm` used by other platforms. The user-facing package is
an installer containing the install, service/client, and uninstall roles.

The Windows single-file bootstrap privately extracts the Qt installer runtime
before Qt code starts because dependent DLLs are resolved at process launch.
The Qt wizard then performs user-selected installation work: runtime placement,
WASM payload, Qt dependencies, KERO home roots, and global preferences.

Windows ARM64 and x64 are independent native targets. They must not assume a
shared compiler runtime or resource compiler merely because both run Windows.

## Why

Treating each architecture as a real native target makes its Qt ABI/toolchain
requirements explicit and prevents an installer from accidentally mixing
runtime libraries from incompatible compiler families.

## Integrity

Windows artifacts receive SHA-256 checksums and receive detached GPG signatures
when a key is configured. Native Windows code signing is currently deferred.
