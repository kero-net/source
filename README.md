# KERO source

This repository is organized by purpose:

- `.github/` contains GitHub-specific configuration and workflows.
- `automation/` contains the Lua automation entry point, tasks, libraries, tests, and schemas.
- `code/` contains the Rust workspace and KERO CLI.
- `publication/` contains channel and release source definitions.
- `localization/` contains locale definitions.
- `assets/` contains non-code project assets.
- `docs/` contains project and manual-page documentation.

Run repository validation with `lua automation/run.lua validation`. All disposable output is written to `.heap/source/`.
