# Stateful Modules

Stateful modules own module-scoped state without independent constructed
instances.

Use sparingly. Prefer clear lifecycle functions when state needs setup,
teardown, or reset behavior.

Use this when there is one shared state owner for the module's domain. It should
feel like a service or cache, not like a half-built object system.

```luau
local Selection = {}

local selectedIds: { [string]: boolean } = {}

function Selection.SetSelected(id: string, selected: boolean)
	selectedIds[id] = selected or nil
end

function Selection.Clear()
	table.clear(selectedIds)
end

return Selection
```

Why this shape:

- the state is shared by the module, not copied per caller
- callers need commands over that shared state
- no `Selection.new()` lifetime would make the ownership clearer

If multiple independent selections need to exist at once, use OOP instead.
