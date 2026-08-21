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
- `.github/actions/` owns GitHub-specific Lua operations.
- `.github/workflows/` composes those operations into GitHub workflows.

Organization-wide GitHub templates and community defaults belong in the
separate `kero-net/.github` repository.

Run repository validation with:

```bash
lua5.4 .github/actions/validate/main.lua
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
