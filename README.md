# KERO source

`kero-net/source` is the canonical authored repository for KERO. The public
`kero-net/kero` repository is generated from this tree; its `canary`, `beta`,
and `stable` branches are never edited directly.

- `code/` owns the Rust workspace, CLI, core library, and manpage.
- `assets/` owns authored images and other shared visual assets.
- `releases/` owns release validation and immutable authored release records.
- `i18n/` owns the ordered locale registry and translation catalogs.
- `pages/` owns the GitHub Pages templates and build system.
- `repo/` owns the generated public-repository projection.
- `.github/actions/` owns reusable GitHub capabilities and the privileged
  publisher.
- `.github/scripts/` owns single-workflow validation entrypoints.
- `.github/tests/` owns workflow and action-wiring contracts.
- `.github/workflows/` composes those operations into GitHub workflows.

Organization-wide GitHub templates and community defaults belong in the
separate `kero-net/.github` repository.

Run repository-contract validation with:

```bash
lua5.4 .github/scripts/repository-contracts.lua
git diff --check
```

Run the independently owned Lua test families with:

```bash
lua5.4 i18n/validate.lua
lua5.4 releases/tests/id.lua
lua5.4 releases/tests/records.lua
lua5.4 releases/tests/sequence.lua
lua5.4 .github/actions/publish/tests/policy.lua
lua5.4 .github/actions/publish/tests/preflight.lua
lua5.4 .github/tests/workflows.lua
lua5.4 .github/scripts/actionlint.lua
```

Build a local Pages preview with:

```bash
lua5.4 pages/build.lua
```

Build a generated public branch with:

```bash
lua5.4 repo/build.lua canary
```

Disposable build and validation output belongs under the workspace `.heap/`.
