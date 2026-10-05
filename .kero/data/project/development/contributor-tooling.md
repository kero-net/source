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
`.heap/build/cargo/default/`. Nested Cargo invocations set an explicit absolute
`CARGO_TARGET_DIR`, so their working directory cannot create a nested heap.
Distribution builds keep native Cargo outputs target-specific beneath
`.heap/build/cargo/` and compile the shared WASM core once.

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
├── build/                         Rust/Cargo and intermediate build output
├── repo/
│   ├── stable/                    generated repository channel
│   ├── beta/                      generated repository channel
│   └── canary/                    generated repository channel
├── pages/                         generated documentation site
├── artifacts/
│   ├── SHA256SUMS                 checksums for the artifact set
│   ├── SHA256SUMS.asc             GitHub-produced detached signature
│   └── distributions/
│       ├── windows-x86_64.exe
│       ├── windows-aarch64.exe
│       ├── linux-x86_64.AppImage
│       └── linux-aarch64.AppImage
└── RELEASE.md                     generated release Markdown
```

Local builds leave `SHA256SUMS.asc` absent because signing belongs exclusively
to GitHub secrets. GitHub creates the same shape and adds the signature. Build
tool downloads and transient toolchain state may live beneath `.heap/build/`
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
`.heap/logs/validate-locally.log` as it proceeds. The record therefore survives
a failed source check, tool acquisition, or package action. Qt's bundled file
logger writes to `.heap/logs/<target>/aqtinstall.log` so independent target
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
beneath `.heap/build/` and are removed after successful publication. The cache
can be discarded with `KERO_CLEAN=1`; its entries are keyed independently of
KERO source changes.

On WSL, the runner uses a workspace-keyed directory under
`~/.cache/kero/workspaces/` on the Linux ext4 filesystem. Linux Qt kits and
extracted toolchains live in its `cache/toolchains/`; Linux Cargo, CMake,
staging, a source snapshot, and the shared WASM compiler output live in its
`build/`. Source validation and Windows packages use the Windows checkout;
Linux compilation reads the ext4 snapshot. Final WASM
and Linux package files are copied into the repository `.heap/build/` for
cross-host packaging and publication. Successful publication removes both
build directories, while `KERO_CLEAN=1` also drops the ext4 cache. Windows
native compiler and packaging paths remain under the repository `.heap/`
because those tools need Windows-visible paths.

On Windows, VS Code launches `distribution/actions/kero-build.ps1`. It runs
source checks with the Windows Lune, Cargo, and Git Bash tools against the
Windows checkout, then invokes the Bash distribution runner in WSL. Linux
compilation reads its ext4 source snapshot. The Bash runner remains the
portable entry point on Linux. It requires the pinned Lune Luau VM in WSL
distribution and reports the exact installation command before work begins if
it is absent. `act` runs the same portable Luau stage
inside a Linux container when both it and Docker are installed. It carries no
secrets or tokens and never substitutes for native desktop evidence. The optional
container check uses an existing local image and reports unavailable parity as
a warning after source-owned contracts pass. `KERO_RUN_ACT=1` explicitly
enables it; ordinary validation skips it. On WSL, the runner can expose an
installed Windows Pandoc through a disposable `.heap/build/bin/pandoc` shim
for Pages generation. If WSL's resolver fails but HTTPS works,
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
