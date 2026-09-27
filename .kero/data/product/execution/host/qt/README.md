# Qt host

**Status:** current replacement design; implementation depends on target Qt
toolchain provisioning.

## Decision

KERO's visual client and installation layer uses **Qt 6 Widgets** above the
terminal-capable native service and one portable `kero.wasm` core.

The visual rule is deliberately narrow: **Qt Widgets plus one external QSS
stylesheet**. There is no QML, web view, generated design framework, embedded
QSS string set, or per-platform visual fork. `kero.qss` is installed beside
the programs. If it is unavailable, normal Qt Widgets rendering remains usable
and the application reports the fallback visibly.

## Why

Qt provides one native desktop UI codebase across the supported host platforms
without moving product semantics out of the WASM core or service. Widgets keep
the client close to native desktop behavior, while an external stylesheet
allows limited KERO presentation without making styling a prerequisite for
functionality.

## Delivered programs

The installation and visual layer exposes three roles:

| Program | Responsibility | Must not do |
|---|---|---|
| `kero-install` | Guided installation, KERO-home selection, existing-home handling, global preferences, runtime placement. | Open a terminal or scan repositories. |
| visual KERO client | Present service-backed home, repository, knowledge, and mount operations. | Reimplement service behavior or treat home data as ambient repository input. |
| `kero-uninstall` | Remove installed runtime and, only after confirmation, a marked KERO home. | Delete an unmarked directory or unrelated repository boundary. |

Windows uses `.exe` suffixes. Other hosts use the same role names without the
Windows suffix where appropriate.

## Current source shape

```text
host/qt/
├── CMakeLists.txt
├── common/
│   ├── CMakeLists.txt
│   ├── runtime_contract.{h,cpp}
│   ├── kero_home.{h,cpp}
│   ├── core_bridge.{h,cpp}
│   └── style_loader.{h,cpp}
├── install/main.cpp
├── app/main.cpp
├── uninstall/main.cpp
└── resources/kero.qss
```

Widget classes do not directly access arbitrary filesystem paths or repository
configuration. They use the scoped service interface; temporary direct
host/core bridging is transition-only and must not become a second behavior
path.

## Home and core contract

The Qt layer uses the marked KERO home defined by
[`../../../environment/home.md`](../../../environment/home.md). It validates
`kero.wasm` through the service before offering product actions and does not
put host paths, CPU/OS facts, mount locators, or credentials in repository
configuration.

## Transition

The earlier NSIS installer and `kero-host.exe` are superseded prototypes. They
remain only until the Qt build produces equivalent artifacts; then their
source, publication hooks, and documentation are removed together so KERO has
one delivery path.

## Reconsider when

Revisit the Qt Widgets choice if a concrete host requirement cannot be met
without introducing a second semantic implementation or an unacceptable
platform/toolchain constraint. A preference for a different UI technology by
itself is not a product-contract change.
