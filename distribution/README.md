# Distribution

This directory owns desktop-package facts and mechanics: target definitions,
toolchain requirements, package assembly, checksums, detached signatures, and
artifact verification. It does not select repository channels or publish.

`actions/kero-build.sh` is the distribution build entry point. It accepts no
target or stage: it builds every enabled distribution, or fails. On Windows,
`actions/kero-build.ps1` performs native source checks before handing the build
to a profile-free WSL Bash process. VS Code exposes this as `Kero: Validate
Locally`.

`builds/*/build.toml` defines one distribution per directory. `tools/*.toml`
defines every external dependency once, including version, location, checksum,
and extraction facts. `scripts/build/build-every-distribution.luau` is the only
executable Luau file; all other Luau code is a module under `lib/`. Lune is the
pinned standalone Luau VM used by the build boundary.

An unchanged completed run verifies its four artifacts and returns. A changed
source rebuilds packages using retained toolchains and compiler intermediates.
After failure, a matching source fingerprint allows checksum-verified completed
targets to resume. No generated file is a required input for a fresh build;
set `KERO_CLEAN=1` to recreate `.heap/` from committed definitions.
When correcting packaging orchestration only, an operator may set
`KERO_REUSE_VERIFIED_ARTIFACTS=1` to reuse checksum-verified artifacts after a
source-fingerprint change; this must not be used after product source changes.

Each target declares its matching native host and toolchain. A package build
must use that exact pairing; compiler families, Qt kits, and runtime deployment
files are not interchangeable across targets. Local paths are contributor
environment configuration and are never committed.

Repository release records and canary/beta/stable publication remain under
`releases/` because they are a separate repository-release subsystem.

## Generated heap contract

The full build produces one clear disposable tree:

```text
.heap/
├── cache/                         downloads, tools, logs, and compiler state
├── pages/                         generated Pages site
├── repo/{stable,beta,canary}/     generated repository channels
├── packages/                      reserved for repository packages
└── release/
    ├── RELEASE-MESSAGE.md
    └── artifacts/
        ├── SHA256SUMS
        ├── SHA256SUMS.asc          GitHub only; absent locally
        └── distributions/
            ├── windows-x86_64.exe
            ├── windows-aarch64.exe
            ├── linux-x86_64.AppImage
            └── linux-aarch64.AppImage
```

Completed release-shaped files belong only in `release/artifacts/`; build directories
must not double as the artifact interface. The local build matches GitHub's
shape but never signs. GitHub uses protected secrets to add the aggregate
checksum signature.

## Local validation

Run **Kero: Validate Locally**. It is the only contributor build command. The
orchestrator validates manifests, runs source and portable checks, prepares
independent target inputs concurrently, and serializes package builds as soon
as each target becomes ready. Each verified package is published immediately;
it does not wait for unrelated targets.
It reports the declared top-level actions as `Building [...] n/N` and writes
the same command/result evidence to `.heap/cache/logs/validate-locally.log`. Target
specific acquisition records live below `.heap/cache/logs/<target>/`.
Downloaded dependency archives and keyed toolchains are retained under `.heap/cache/`;
Cargo output and package staging are working state under `.heap/cache/build/` and
are removed after successful publication. Preparation
is dependency-aware and concurrent; package compilation remains serialized.
If a later target fails, earlier verified artifacts and their checksums remain,
and `release/RELEASE-MESSAGE.md` marks the run incomplete.

The validation entry point bootstraps a pinned `act` release under `.heap/cache/`
and replays the GitHub CI orchestration from an isolated source snapshot. It
does not rebuild distribution packages inside `act`: package creation runs
exactly once through the source-owned distribution modules that GitHub package
jobs also invoke. The replay records a source fingerprint, and the native build
refuses to continue if the checkout changes between workflow replay and package
creation. The replay carries no signing secret or publication token; its logs
and resolved source and tool versions stay under `.heap/cache/logs/`.

This deliberately treats GitHub as the workflow reference while avoiding a
second CI implementation. `act` proves Linux-hosted workflow structure,
conditions, actions, and artifact-oriented CI behavior on the contributor host
architecture. Platform package providers remain responsible for the real
Windows and Linux toolchains required by each target. Exact GitHub runner-image
identity is not claimed when the contributor machine has a different
architecture; target semantics and package commands remain shared source.

For a Windows ARM64 package, the wrapper requires the Qt `msvc2022_arm64` kit
and the matching Visual Studio ARM64 target compiler. It refuses to use an x64
compiler for that package because the resulting bootstrap and DLLs would not
be native ARM64.

`propose` requires a clean committed revision and native runtime evidence for
every enabled target. It
writes a reviewable candidate manifest under `distribution/candidates/`; the
packages themselves remain local. Maintainers reproduce approved candidates on
native hosts and sign only from their protected local key environment.

Contributor commands never require credentials, tokens, or signing keys. A
package is unsigned by default even if a key fingerprint exists in the shell.
When GitHub's protected `release` environment provides a private key and
passphrase, publication sets `KERO_SIGN_RELEASE=1` and signs packages, commits,
and tags. Without those secrets, publication uses unsigned packages, commits,
and annotated tags. A partial private-key configuration fails before packaging.
Local and pull-request outputs remain unsigned.

Windows ARM64 cross-builds the configured LLVM-MinGW Windows x64 target.
Downloaded inputs remain disposable under `.heap/`.
