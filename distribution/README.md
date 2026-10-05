# Distribution

This directory owns desktop-package facts and mechanics: target definitions,
toolchain requirements, package assembly, checksums, detached signatures, and
artifact verification. It does not select repository channels or publish.

`actions/kero-build.sh` is the only executable distribution entry point. It
accepts no target or stage: it builds every enabled distribution, or fails.
On Windows, VS Code launches this same file directly through a profile-free
WSL Bash process. VS Code exposes exactly that command as `Kero: Validate
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
├── build/                         expanded tools and compiler intermediates
├── repo/{stable,beta,canary}/     generated repository channels
├── pages/                         generated pages
├── logs/                          complete run and target-scoped logs
├── artifacts/
│   ├── dependencies/              verified downloaded build inputs
│   ├── SHA256SUMS
│   ├── SHA256SUMS.asc             GitHub only; absent from unsigned local builds
│   └── distributions/
│       ├── windows-x86_64.exe
│       ├── windows-aarch64.exe
│       ├── linux-x86_64.AppImage
│       └── linux-aarch64.AppImage
└── RELEASE.md
```

Completed release-shaped files belong only in `artifacts/`; build directories
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
the same command/result evidence to `.heap/logs/validate-locally.log`. Target
specific acquisition records live below `.heap/logs/<target>/`.
Downloaded dependency archives and keyed toolchains are retained under `.heap/cache/`;
Cargo output and package staging are working state under `.heap/build/` and
are removed after successful publication. Preparation
is dependency-aware and concurrent; package compilation remains serialized.
If a later target fails, earlier verified artifacts and their checksums remain,
and `RELEASE.md` marks the run incomplete.

`act` is optional Linux-container parity, enabled only with `KERO_RUN_ACT=1`.
It contains no secret, token, or local state.

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
