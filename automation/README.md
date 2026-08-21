# Automation implementation

The supported entry point is `automation/run.lua`. Lua modules under `tasks/`
implement individual operations, while reusable primitives live under `lib/`.
Configuration and schemas are colocated here because they describe automation,
not GitHub. No language-specific directory or Python environment is required.
