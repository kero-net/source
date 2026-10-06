# Contributor tooling

**Status:** active.

The source repository must own the shared VS Code contributor experience for
Rust, Qt/C++, CMake, and Luau work. Opening the authoritative `source/`
repository should expose the appropriate extension recommendations and the
safe, portable build entry points without requiring a contributor to discover
local coordination-folder configuration.

## Current state

`source/.vscode/extensions.json` recommends Rust Analyzer, C/C++, CMake Tools,
and Luau support. `.heap/` is ignored within `source/` so disposable output
follows the repository when it is opened directly. VS Code exposes one standard
task, `Kero: Validate Locally`, which calls the source-owned runner.
The source-owned `.cargo/config.toml` directs ordinary Cargo output to
`.heap/cache/build/cargo/default/`. Nested Cargo invocations set an explicit absolute
`CARGO_TARGET_DIR`, so their working directory cannot create a nested heap.
Distribution builds keep native Cargo outputs target-specific beneath
`.heap/cache/build/cargo/` and compile the shared WASM core once.

## Direction

The immediate return-to priority is a full build that publishes every enabled
artifact. `Kero: Validate Locally` verifies a completed unchanged artifact set
quickly. After a failed run it retains downloaded inputs and, when the source
fingerprint matches, verifies and skips completed artifacts. If source changed,
it rebuilds packages using retained toolchains and compiler intermediates.
`KERO_CLEAN=1` forces a fresh run. Nothing required to
reproduce a build may exist only in `.heap/`;
scripts, dependency versions, target definitions, and bootstrap instructions
remain committed source.

Full-build evidence is the completed four-artifact manifest and its checksums.
Until every target passes package verification, the run remains incomplete.

The canonical generated layout is:

```text
.heap/
├── cache/                         downloads, tools, logs, and build output
├── repo/
│   ├── stable/                    generated repository channel
│   ├── beta/                      generated repository channel
│   └── canary/                    generated repository channel
├── pages/                         generated documentation site
├── packages/                      reserved for repository packages
└── release/
    ├── RELEASE-MESSAGE.md         generated release Markdown
    └── artifacts/
        ├── SHA256SUMS             checksums for the artifact set
        ├── SHA256SUMS.asc         GitHub-produced detached signature
        └── distributions/
            ├── windows-x86_64.exe
            ├── windows-aarch64.exe
            ├── linux-x86_64.AppImage
            └── linux-aarch64.AppImage
```

Local builds leave `SHA256SUMS.asc` absent because signing belongs exclusively
to GitHub secrets. GitHub creates the same shape and adds the signature. Build
tool downloads and transient toolchain state may live beneath `.heap/cache/build/`
during one run, but the run must bootstrap them again after the heap is deleted.
Target-specific requirements remain canonical under
[`product/distribution/`](../../product/distribution/).

Qt acquisition installs the `qtbase` archive for each target kit and the
`icu` runtime archive for Linux kits, whose Qt host tools require ICU 73. The host
CMake project links Qt6 Network and Widgets, both provided by Qt Base; the
Windows deployment tool is supplied by the matching Qt Base host kit. No Qt
WebEngine, PDF, Wayland, Qt Tools, or translation module is requested. The
installer copies a translation plugin directory if a deployment tool supplied
one; it does not make that module a build dependency.

The runner presents every declared top-level action as `Building [...] n/N` on
one persistent terminal line and writes a complete command/result record to
`.heap/cache/logs/validate-locally.log` as it proceeds. The record therefore survives
a failed source check, tool acquisition, or package action. Qt's bundled file
logger writes to `.heap/cache/logs/<target>/aqtinstall.log` so independent target
preparation never races for one log file. Preparation follows its declared
dependency graph concurrently: independent toolchains acquire together, while
`linux-x64` waits for the ARM64 Qt host kit it explicitly consumes. Builds
remain sequential because their Cargo and toolchain locations are shared
clean-room evidence, not a safe parallel work queue.

The terminal also streams each command's output as it is produced. The runner
prints the command and scope before execution, and reports source-owned file
reads and directory listings by path. During an interactive run, one progress
line is rewritten in place using carriage return. Before a child emits output,
the runner clears that line, lets the output scroll normally, then redraws the
current progress. The same command output is appended to its stage
log and to `validate-locally.log` so the live view and diagnostic record agree.
Source checks announce each check group; repository and localization checks
print the exact paths they inspect.

The runner's terminal vocabulary uses a single progress line at column zero,
four spaces before runner action labels, and eight spaces for nested scope,
command, and child-output lines. `::` names a scope, `+` marks a command that
ran, and `-` marks omitted optional work. Loading and progress arrows appear
only inside square-bracket progress blocks. Labels carry meaning: cyan for starting/running, green
for completion, yellow for warnings/skips, red for failures, dim for file reads
and listings, and magenta for progress. Color is enabled for interactive runs,
disabled for redirected output and `NO_COLOR`, and never written to run logs.
`Kero: Preview Terminal Styles` prints sample labels, command output, and the
indentation legend without changing `.heap/` or starting validation. It uses
the host's Lune installation, so Windows contributors can preview without a
second Lune installation in WSL. The same preview is available with
`lune run distribution/scripts/preview-terminal.luau`.

Downloaded, checksummed dependency inputs are retained beneath
`.heap/cache/downloads/`. On non-WSL hosts, versioned Qt kits and host tools
live beneath `.heap/cache/toolchains/`, with a shared pip cache at
`.heap/cache/pip/`. On WSL those extracted tools and the pip cache live in the
ext4 cache described below. `aqtinstall` is installed once per version and
host Python ABI.
Cargo targets, CMake output, package staging, and release handoff files belong
beneath `.heap/cache/build/` and are removed after successful publication. The cache
can be discarded with `KERO_CLEAN=1`; its entries are keyed independently of
KERO source changes.

On WSL, the runner uses a disposable workspace-keyed directory under
`${TMPDIR:-/tmp}/kero/workspaces/` on the Linux ext4 filesystem. Linux Qt kits and
extracted toolchains live in its `cache/toolchains/`; Linux Cargo, CMake,
staging, a source snapshot, and the shared WASM compiler output live in its
`build/`. Source validation and Windows packages use the Windows checkout;
Linux compilation reads the ext4 snapshot. Final WASM
and Linux package files are copied into the repository `.heap/cache/build/` for
cross-host packaging and publication. Successful validation removes the ext4
work directory; `KERO_CLEAN=1` drops retained state from a failed run. Windows
native compiler and packaging paths remain under the repository `.heap/`
because those tools need Windows-visible paths.

On Windows, VS Code launches `distribution/actions/validate-locally.ps1`. It
uses the pinned `act` release to replay GitHub CI orchestration from a
disposable source snapshot, then invokes `kero-build.ps1` exactly once for the
four distribution packages. Package jobs are not duplicated inside `act`.
The workflow replay records the source fingerprint and the package build refuses
to start if the source changes afterward. Missing workflow prerequisites,
failed CI jobs, failed package providers, or mismatched artifacts stop
validation. `act` carries no signing credentials and cannot represent native
Windows or macOS runtime evidence. On WSL, the runner can expose an installed
Windows Pandoc through a disposable `.heap/cache/build/bin/pandoc` shim for
Pages generation. If WSL's resolver fails but HTTPS works,
`KERO_CURL_DOH=1` enables DNS over HTTPS for pinned `curl` downloads without
changing expected checksums.

## Configuration ownership

New human-editable KERO-owned operational settings use KERO Structured Text
(KST). Every mutable default, policy, toolchain, or release value has one
authoritative editable definition; scripts and generated records consume or
derive it rather than carrying a second default. Migrate existing KERO-owned
TOML settings incrementally with changes to their owning subsystem. Keep TOML
only where an external tool requires it, including Cargo manifests.

The current ownership inventory and migration sequence are recorded in
[`kst-operational-settings.md`](kst-operational-settings.md).

Review changes for configuration ownership and add focused contract coverage
for each migrated setting schema and consumer. Do not use a generic duplicate
literal scan: matching text is not proof of duplicate policy.

## Replacement record

This document replaces the implicit assumption that local VS Code tasks are
the package entry point and the prior coupling of every build to a runtime
test. The source repository is the actual clone/open boundary; its remote
pipeline and disposable output live there.

## Next evidence

Verify that two consecutive full runs both begin from no heap, recreate the
canonical tree, produce all enabled unsigned distributions and aggregate
checksums, and reject a candidate proposal until complete native evidence exists.

## Development feedback and performance evidence

Do not use the complete distribution workflow as the primary development
feedback loop. Make changes using static inspection and targeted tests first.
Run the smallest relevant validation command for the component being modified.
Only run the complete multi-target distribution workflow when all targeted
checks pass or when full integration behavior specifically needs validation.

When continuing from a failed build, report both the continuation time and the
cumulative elapsed work required from the start of the original build state.
Cached artifacts produced by failed attempts are not an optimization unless
the same cache is intentionally reusable from a clean documented starting
state. Performance comparisons require a controlled cold build and a controlled
warm build; a resumed or partially populated `.heap/` is not evidence of a
build-time improvement.
The complete four-target build has a 30-minute wall-clock budget. Record the
controlled cold and warm timings separately and treat a cold run at or above
30 minutes as an unresolved performance failure.

Do not improve build time by merely retaining more generated state. Any caching
change must have an explicit cache key, ownership/lifecycle, and cleanup policy.
The goal is both faster builds and bounded `.heap` growth.

## Source automation runtime

The source-owned release, repository, Pages, localization, and GitHub action
helpers use `.luau` modules and the pinned Lune 0.10.5 runtime. Workflows and
local source checks invoke them with `lune run`; Lua 5.4 is no longer a source
automation prerequisite. The source helper adapter under
`.github/actions/lib/legacy-runtime.luau` preserves the existing file and
process behavior while these subsystems move to direct Lune APIs.

The pinned Lune release version and per-platform SHA-256 digests are owned by
`distribution/tools/lune.toml`. GitHub jobs that execute Luau install the
verified prebuilt release through the source-owned setup action into runner
temporary storage. Local WSL validation installs it into the single disposable
`.heap/cache/toolchains/lune/` slot, replacing the previous version in place;
clearing `.heap` removes it. Rust, manpage, and dependency-input checks run
directly and do not require Lune. This replaces compiling Lune separately in
each workflow job and during local bootstrap.

Distribution build manifests name the hosted runner and packaging inputs.
They do not select a Lua interpreter; the hosted matrix and portable checks
use the same pinned Lune runtime as local validation.

Hosted publication derives compiler-family and packaging inputs from those
manifests instead of inferring them from the runner operating system. The
publication workflow builds the shared WASM core once, then supplies it to each
package job. Hosted runners may use a native execution strategy even when local
validation uses a cross-build strategy for the same target: Windows x64 uses
the pinned x86_64 LLVM-MinGW archive on the x64 runner, Linux x64 uses native
GCC on the x64 runner, Windows ARM64 binds Cargo and CMake to the ARM64 MSVC
toolchain explicitly, and Linux package jobs use the manifest-selected
linuxdeploy architecture and AppImage runtime. Target semantics remain owned by
the distribution manifests; host-specific execution details must not redefine
the target.

The hosted Windows Rust setup runs a source-owned PowerShell bootstrap rather
than invoking a Bash shell. This avoids resolving `bash.exe` to WSL on a native
Windows ARM64 runner. The bootstrap installs pinned Rust, requested targets, and
components with rustup, placing its download under runner temporary storage.
All package jobs pass the downloaded shared WASM core by an absolute workspace
path. Windows package configuration copies the generated icon into the CMake
build directory and puts that directory on the resource compiler include path.


## Validation parity boundary

GitHub workflow YAML is authoritative for orchestration, permissions,
conditions, and release-only side effects. Source-owned distribution modules
are authoritative for package mechanics. Local validation must execute those
same package modules rather than maintain a second translation of the workflow.

The local gate therefore has two non-overlapping phases:

1. replay CI orchestration with `act` against an immutable source snapshot;
2. execute the real target providers once and verify the four release-shaped
   artifacts.

Signing, publication tokens, repository mutation, and release-channel writes
exist only in GitHub. A local pass does not claim byte-for-byte equivalence with
a different GitHub runner image or native runtime evidence for an unavailable
host. It does prove that the checked source passed the workflow gate and the
same source-owned package implementation before a publication run is attempted.

Do not add local-only replicas of inline GitHub package logic. Substantial
package behavior belongs in source-owned modules or scripts consumed by both
entry points. If a workflow needs behavior that cannot be called locally, move
that behavior behind a source-owned boundary before extending the local gate.
