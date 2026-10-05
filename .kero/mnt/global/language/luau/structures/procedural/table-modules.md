# Table Modules

A table module exposes related functions through a module identity table.

Use it when the module has a real surface area: multiple operations, a domain
name worth keeping in call sites, or helpers that belong together without
becoming independent objects.

Required:

- module identity table
- public functions attached to that table
- return of the module table

Usually absent:

- `.new`
- metatable instance type
- fake private methods

```luau
-- src/app/Render/Text.luau
--!strict

local Text = {}

function Text.Measure(value: string): number
	return #value
end

return Text
```

Why this shape:

- `Text` names the operation family
- callers choose among public functions on the same module
- no constructed instance is needed
- local helpers can stay private without fake table-private methods
