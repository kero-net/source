# Stage 09 — Application and CLI integration

**Planned.** Connect the terminal service, CLI, and Qt client into one
coherent product surface.

This stage replaces the former cross-platform release stage. It moved because
platform packages are only useful after both application entry points expose
the same completed behavior. The CLI and Qt client must share service
semantics, identity, errors, configuration, and lifecycle behavior without
duplicating product logic.

## Completion evidence

- Equivalent supported operations have consistent results in the CLI and app.
- Configuration and service lifecycle changes are visible to both clients.
- Help, diagnostics, and error messages identify the correct recovery action.
- The app and CLI can be exercised together in one local environment.

Functional testing follows in stage 10. Native-platform proof now belongs to
stage 12 and publication to stage 13.
