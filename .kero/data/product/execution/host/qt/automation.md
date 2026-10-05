# Local Qt UI automation

**Status:** experimental host tooling, captured 2026-09-20.

KERO may expose an explicitly enabled, local-only automation endpoint so UI
test runners and agents can inspect and operate the real Qt Widgets window.
This is host tooling, not a KERO-core capability.

## Portability model

The application implements the endpoint with Qt APIs and one portable JSON
protocol. It therefore shares source across Windows, Linux, and CPU
architectures supported by the Qt host. An optional MCP server can be a thin
stdio client of this endpoint. Native accessibility integrations, if later
needed for applications other than KERO, remain adapters behind the same
client contract.

## Safety boundary

- The endpoint is disabled by default.
- It binds only to the local loopback interface.
- Startup requires both `--automation-port` and a non-empty
  `--automation-token`.
- Every request must present the exact token.
- Widgets are addressed by Qt `objectName`, not screen coordinates.
- The initial command set is intentionally narrow: inspect, screenshot, click,
  selectRow, and explicitly supported state changes.

## Protocol

One UTF-8 JSON object is sent per TCP connection. KERO replies with one JSON
object followed by a newline, then closes the connection.

```json
{"token":"local-secret","method":"inspect"}
{"token":"local-secret","method":"screenshot"}
{"token":"local-secret","method":"click","target":"refreshOverviewButton"}
{"token":"local-secret","method":"selectRow","target":"navigationList","row":2}
```

`inspect` returns the window and recursively described widgets. `screenshot`
returns base64 PNG data. `setChecked` changes a named checkbox or radio button.
Mutating commands reject missing, disabled, or incorrectly typed targets.

## Why

Automation should exercise the same real UI shipped to users without granting
an ambient remote-control interface. Loopback-only transport, explicit startup,
a per-run token, and semantic widget names keep the testing capability narrow
and deterministic.
