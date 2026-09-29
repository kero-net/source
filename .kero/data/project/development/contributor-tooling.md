# Contributor tooling

**Status:** active.

The source repository must own the shared VS Code contributor experience for
Rust, Qt/C++, CMake, and Lua work. Opening the authoritative `source/`
repository should expose the appropriate extension recommendations and the
safe, portable build entry points without requiring a contributor to discover
local coordination-folder configuration.

## Current state

`source/.vscode/extensions.json` recommends Rust Analyzer, C/C++, CMake Tools,
and Lua support. `.heap/` is ignored within `source/` so disposable output
follows the repository when it is opened directly. VS Code exposes one standard
task, `Kero: Validate Locally`, which calls the source-owned runner.

## Direction

The immediate return-to priority is a clean-room full build. `Kero: Validate Locally`
must treat `.heap/` as disposable output, delete and recreate it at the start of
each run, and then produce the complete unsigned local representation of the
GitHub build. Nothing required to reproduce a build may exist only in `.heap/`;
scripts, dependency versions, target definitions, and bootstrap instructions
remain committed source.

The current runner does not meet that contract: it reaches the enabled
`linux-x64` target and fails because its WSL builder is unimplemented. This is
an unresolved build failure, not partial or successful coverage. Resume work by
making every enabled target build from a freshly recreated heap.

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

Downloaded, checksummed dependency inputs are retained beneath
`.heap/artifacts/dependencies/`. Extracted toolchains, Cargo targets, package
staging, and release handoff files belong beneath `.heap/build/` and are
removed after their final artifacts have been published. This keeps one durable
disposable record of each downloaded input without retaining its much larger
expanded working tree.

`distribution/actions/kero-build.sh` is the single Bash contributor entry
point; VS Code launches it through WSL on Windows and never routes validation
through PowerShell. The runner requires the pinned Lune Luau VM in that WSL
distribution and reports the exact installation command before work begins if
it is absent. `act` runs the same portable Luau stage
inside a Linux container when both it and Docker are installed. The repository
`.actrc` supplies the shared image mapping but no secrets, tokens, or automatic
installation; it never substitutes for native desktop evidence.

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
