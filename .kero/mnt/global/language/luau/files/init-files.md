# Init Files

`init.luau` is a folder head, not a decoration.

Use it when the folder owns one of these:

- a public contract for a runtime, tool, app, or subsystem
- validation or normalization at the boundary
- lifecycle and setup/teardown
- coordination between private child modules
- a stable facade used by callers

Avoid placeholder folder heads that only mirror child modules.

```text
Render/
  init.luau
  Text.luau
  Layout.luau
```
