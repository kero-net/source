# Qt application design

**Status:** current context-centered service-client design.

These refinements change host presentation only. They do not change KERO core
semantics, repository/mount rules, or the platform-native Qt styling policy.

## Current priorities

The visual client has one explicit selected folder, shown below the Overview
heading rather than treated as global application chrome. It records only
recent user-selected paths in local Qt settings, then asks `kero-host` to
classify each path before presenting it. A **KERO context** is the service
classification of that folder, not the folder text itself:

- **KERO Home** applies only when the selected folder is exactly `KERO_HOME`.
- **Eligible repository** is a Git worktree that may be explicitly enrolled.
- **Enrolled repository** is a worktree governed by its nearest valid `.kero`.
- **No KERO context** is an ordinary folder. It is an honest inactive state,
  not an error; Home/repository operations remain unavailable.
- **Unavailable** is an actual classification or service failure and gives an
  actionable error.

The application never infers repository state by scanning its launch directory
or a `.kero` tree. Statuses use one fixed-size painted dot component with text,
rather than Unicode circle characters whose size varies with the font. The
presentation maps semantic status (`neutral`, `info`, `success`, `warning`,
`error`) through Qt's live platform palette. Native surface, text, disabled,
and accent colors therefore follow Windows, macOS, and Linux themes; only the
portable success/warning/error hue is adapted for contrast where Qt supplies
no semantic role. Color never stands alone as the status meaning.

Knowledge is a service-owned data-scope browser. Local and named mounted scopes
are distinct, and a **mount** is the explicit addition of knowledge from an
already existing KERO environment into the current enrolled repository. It is
not creation of a new `.kero/` boundary, and mounts have no precedence.
The Qt client obtains context, scope entries, mount provenance/status, and all
mutations through `kero-host`; it does not parse private IPC runtime state or
reimplement filesystem classification.

The current **Index** tab is a visual placeholder and is unrelated to the
representation-machine research track. It must not imply that a reconstruction
representation, semantic index, or user-facing query contract exists before a
separate product contract defines one.

1. Present Home as a separate context with global data and settings.
2. Present eligible repositories with an explicit **Create KERO environment**
   action; it is repository enrollment, never mounting, and never occurs solely
   because a user selected a location.
3. Present enrolled repositories with service-backed local knowledge and named
   mount management.
4. Keep mount addition explicit: select source, inspect/classify, name, then
   confirm materialization. Refresh, event mode, sync, conflict resolution, and
   removal remain service operations.

## Earlier visual priorities

1. Remove redundant frog artwork from Overview and Knowledge. Retain the
   sidebar brand icon; use mascot art only for purposeful empty, first-run,
   success, or About states.
2. Use one status-indicator vocabulary throughout the app:
   - green `● Ready`
   - yellow `● Rebuild needed`
   - red `● Mount unavailable`
   - system-colored `○ Disabled`
3. Constrain ordinary page content to an approximately 850–1000 px maximum
   width, aligned left. Knowledge and Mount tables may use available width.
4. Increase sidebar navigation row density and selection weight while retaining
   native Qt appearance. Add small monochrome section icons later.
5. Reduce Overview to environment identity, health, and changes. Move verbose
   path data to Settings or an expandable details area.
6. Flatten Overview nesting: show repository name/path as a header above useful
   status summaries instead of surrounding them with nested groups.
7. Treat Mounts as primary management UI. Evolve rows toward clear name,
   status, source, destination, enabled state, and order presentation.
8. Retain the Knowledge browser's tree/details structure while removing its
   redundant mascot artwork.
9. Present Policy as understandable state and future actions, for example:
   Verification: Automatic; Identity: Reuse existing identity; Repository
   enrollment: Ask before enrolling. Add change controls only once backed by
   the shared host/core bridge.
10. Keep Settings for inspectable or editable settings only. Move the
    non-configurable Qt theme explanation to About or documentation.
11. Keep About compact: version, build/revision, and later source/license links.
12. Correct repository detection before relying on Overview or Mounts details.
    The selected environment must resolve to the actual nearest repository or
    KERO boundary, never a Windows 8.3 path alias mistaken for a repository.
13. Do not introduce a KERO palette. Qt/Windows owns normal background,
    controls, focus, and selection; KERO-specific color is reserved for status.

## Navigation hierarchy

```text
Sidebar
├── Overview       compact status
├── Knowledge      browser
├── Mounts         primary management UI
├── Policy         configuration/state
└── Settings       environment/app settings
```

## Why

The app should expose KERO state and actions without inventing a second visual
system or masking incorrect repository state with polish. Product correctness
and shared host/core wiring therefore precede UI that depends on them.

## Sequencing

Address repository discovery and shared host/core operation wiring before
polishing data that currently reflects placeholder discovery. Visual-only work
may proceed independently when it cannot conceal incorrect state.
