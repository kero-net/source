# KERO source

`kero-net/source` is KERO's authored source repository. GitHub reviews source
branches here. The public `kero-net/kero` repository is generated from this
tree; do not edit its `canary`, `beta`, or `stable` branches directly.

## Which command to run

| Goal | Command | Result |
| --- | --- | --- |
| Check source and workflow contracts on a branch | `lua .github/scripts/repository-contracts.lua` | Source-only validation; no desktop package. |
| Check portable distribution contracts | `kero.cmd portable` on Windows, `./kero portable` on Linux/macOS | Lua checks; runs the local Actions workflow through `act` when available. |
| Run the portable Actions workflow directly | `act -j portable` | The same portable Lua stage in a Linux container. |
| Create a local package | `kero.cmd build current` on Windows, `./kero build current` on Linux/macOS | Unsigned package, checksum, and static validation under `.heap/`. |
| Validate an existing local package | `kero.cmd verify TARGET` or `./kero verify TARGET` | Checksum, package layout, and target architecture only. |
| Test a package runtime | `kero.cmd test TARGET --adapter native` or `./kero test TARGET --adapter native` | Bounded startup and bootstrap-payload checks; records execution mode. |
| Build all locally available targets | `kero.cmd all current` or `./kero all current` | Native host test plus cross-builds; reports incomplete release evidence without failing development work. |

The wrappers invoke Lua internally. Direct `lua distribution/scripts/local-run.lua ...`
is supported for automation, but contributors normally use `kero.cmd` or
`./kero`.

## Branch checks, package checks, and releases

Branch checks answer whether authored source, contracts, and workflow wiring
are internally consistent. `act` is useful here because it reproduces the
portable Linux workflow. It does **not** create a Windows or macOS virtual
machine, so it cannot establish native desktop-package evidence.

Package checks produce artifacts in `.heap/build/releases/<target>/`. A build
proves compilation, package layout, checksum integrity, and architecture. A
runtime test additionally proves that the bootstrap payload extracts and the
packaged application reaches its startup state. Evidence records whether that
execution was `native`, `emulated`, or a configured `vm`.

A release candidate is stricter: `kero propose` requires a clean committed
revision and native runtime evidence for every enabled target. Cross-builds and
Windows ARM x64 emulation are valuable diagnostics but never become native
release evidence. Maintainers reproduce candidates and sign only in their
protected local release environment.

## Install only what your command needs

All contributors need Lua 5.4, CMake, Rust with `wasm32-wasip1`, and Git for
Windows' POSIX shell on Windows. No GitHub CLI, token, signing key, Docker, VM,
or clean working tree is required for ordinary local builds.

Windows ARM64 builds additionally need Qt `msvc2022_arm64`, Visual Studio 2022
C++ ARM64 tools, and a Windows SDK. Windows x64 cross-builds on Windows ARM64
need Qt `llvm-mingw_64` plus LLVM-MinGW x86_64 tools and their runtime DLLs.
The wrapper discovers these installations; it never commits their paths.

`act` and Docker are optional and only add portable/Linux-container checks.
Hyper-V and QEMU are optional local adapters configured in ignored
`distribution.local.kst`; they never download guest images or credentials.
Linux x64 packages run directly on a native x64 Linux distribution; Ubuntu is
one supported example, not a special runner requirement. The POSIX wrapper finds
`lua`/`lua5.4`, `qtpaths6`, and `linuxdeploy` from `PATH`, or reports precisely
which of those packages are absent. macOS ARM64 is **coming soon** and needs a
Mac contributor with Xcode, Apple tools, and a matching Qt kit.

## Repository layout

- `src/` — Rust workspace, CLI, core library, and manpage.
- `distribution/` — package contracts, local runner, adapters, and tests.
- `releases/` — immutable release records and publication validation.
- `.github/` — reusable GitHub capabilities, workflow contracts, and release wiring.
- `.kero/` — canonical product and contributor-workflow knowledge.
- `.heap/` — ignored local artifacts, diagnostics, and evidence.

Read `.kero/README.md` and the applicable canonical node before changing
product behavior, packaging, or contributor workflow.
