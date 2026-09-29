# Luau analysis support

This directory supplies the small, repository-owned type surface used to
analyze KERO's Lune build code. It deliberately contains declarations only:
the Lune runtime itself is installed by the build bootstrap and is never
committed here.

Run the distribution analyzer with the Luau language server installed by the
editor:

```powershell
luau-lsp analyze --platform=standard --definitions .luau-lsp/standard.d.luau --base-luaurc .luaurc distribution
```

`--platform=standard` is required because distribution code targets Lune, not
Roblox. The root `.luaurc` resolves `@lune/*` imports to the declarations in
this folder, so diagnostics are reproducible on every contributor machine.
