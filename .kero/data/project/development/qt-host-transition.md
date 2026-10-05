# Qt visual-client transition

The current Qt implementation and the earlier NSIS installer/`kero-host.exe`
prototype are transitional paths. The durable direction is a terminal-capable
KERO service with a Qt visual client above it. Until that service/client shape
produces equivalent artifacts, the prototype remains transition-only.

The current Windows prototype runtime installs:

```text
<install>/runtime/
├── kero-host.exe
└── kero.wasm
```

Its explicit CLI operations are:

```text
kero-host repository status [path]
kero-host repository enroll --policy ask|automatic|manual [path]
kero-host mount add <name> <source> [--repository <path>]
kero-host mount remove <name> [--repository <path>]
```

`kero-host.exe` locates its sibling `kero.wasm` by default. Development and
embedding may override that location with `--runtime <path>` or `KERO_RUNTIME`.
These are host/runtime controls and never belong in repository configuration.

## Why this is project knowledge

These names and packaging details describe a migration state. The durable
product contract is the portable core plus native service boundary under
[`../../product/execution/`](../../product/execution/), while the Qt visual
client contract lives under [`../../product/execution/host/qt/`](../../product/execution/host/qt/).

When the service and Qt visual client produce equivalent install/app/uninstall
artifacts, remove the superseded prototype source, publication hooks, and this
transition record together.
