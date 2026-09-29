# OOP Objects

This document applies to project-owned, metatable-backed objects with
independently constructed instances.

It does not apply to return-function modules, plain table modules, ECS
components, ECS systems, data modules, entry points, or engine-owned Roblox
objects.

Use OOP when the file needs to preserve instance identity: a widget owns a root
frame, a controller owns connections, or a runtime object needs a clear destroy
path. Do not use it just to group functions under a nice name.

## Constructor Strategy

Constructor shape is a decision, not ceremony.

Use fields-first construction when all required fields are known before the instance needs to exist:

```luau
type LoadingScreenFields = {
	Gui: ScreenGui,
}

export type LoadingScreen = setmetatable<LoadingScreenFields, typeof(LoadingScreen)>

function LoadingScreen.new(playerGui: PlayerGui): LoadingScreen
	local fields: LoadingScreenFields = {
		Gui = findExistingGui(playerGui) or createGui(playerGui),
	}

	return setmetatable(fields, LoadingScreen)
end
```

Use incremental `self` when setup is staged, conditional, cyclic, or
callback/helper driven:

```luau
local Widget = {}
Widget.__index = Widget

type WidgetFields = {
	Root: Frame,
	Connections: { RBXScriptConnection },
}

export type Widget = setmetatable<WidgetFields, typeof(Widget)>

function Widget.new(): Widget
	local self = setmetatable({}, Widget) :: Widget

	self.Root = Instance.new("Frame")
	self.Connections = {}

	return self
end
```

Use one constructor-boundary cast only when construction order genuinely needs it. Do not use call-site casts to compensate for constructor typing.

Prefer fields-first construction when possible. Use empty-table incremental
construction when the instance is built out step by step.

Do not inline a populated field table inside `setmetatable`. Use one of the
established construction shapes.

Fields-first:

```luau
local fields: WidgetFields = {
	Root = createRoot(),
	Connections = {},
}

local self = setmetatable(fields, Widget)
```

Empty-table staged:

```luau
local self = setmetatable({}, Widget) :: Widget
self.Root = Instance.new("Frame")
self.Connections = {}
```

The empty-table staged form is less type-safe. Keep the cast at the constructor
boundary and do not spread it to callers.

## Method Declarations

Declare project-owned receiver methods with dot syntax and explicit `self`:

```luau
function Widget.Destroy(self: Widget)
	self.Root:Destroy()
end
```

Call concrete receiver methods with colon syntax:

```luau
widget:Destroy()
```

This keeps receiver typing explicit while preserving normal call ergonomics.

Roblox APIs and engine objects keep their native colon-call shape.

Do not use a metatable-derived concrete instance type as a shared
cross-implementation interface. Shared abstractions should use an explicit
structural contract.
