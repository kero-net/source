# Organization repository

`kero-net/.github` owns the organization profile, community templates, workflow
templates, and reusable workflows.

Repository-specific triggers and configuration stay in the repository that
owns the work. Shared workflow logic may be called from reusable workflows in
`kero-net/.github`; workflow templates only help create local workflows and do
not execute automatically.

## Why

Organization defaults should be shared without hiding which repository owns a
trigger, permission boundary, or release action.
