# Luau Naming

This document owns Luau naming, variables, constants, and local identifier
constraints.

## camelCase

Use `camelCase` for:

- ordinary locals
- parameters
- private helper functions
- instance fields
- data table keys and nested data fields
- module-scoped mutable state

```luau
local activeConnections = {}
local function resolveTargetPlayer(text: string): Player?
end

return {
	spacing = {
		small = 4,
		medium = 8,
	},
}
```

## PascalCase

Use `PascalCase` for:

- Roblox services
- required module bindings
- module identity tables
- exported types
- local type aliases

```luau
local Players = game:GetService("Players")
local Renderer = require("./Render/Renderer")

type ActionContext = {
	player: Player,
}

local CommandRunner = {}
```

Do not use `PascalCase` for data table keys or ordinary record fields. Those
fields remain `camelCase` even when the containing table is exported from a
module.

## Public Functions And Methods

Public module functions and methods use `PascalCase`.

`.new` is the constructor exception.

```luau
function CommandRunner.new(): CommandRunner
end

function CommandRunner.Destroy(self: CommandRunner)
end

function CommandRunner.RunCommand(...)
end
```

## Constants

Constants are immutable values that describe behavior, policy, identifiers, limits, or fixed defaults.

Constants may use either `SCREAMING_SNAKE_CASE` or `camelCase`, depending on
what kind of constant they are.

Use `SCREAMING_SNAKE_CASE` for policy, identity, limits, and fixed defaults:

```luau
const DEFAULT_FADE_TIME = 0.25
const MAX_RETRY_COUNT = 3
const FONT_METADATA_KEY = "FontMetadata"
```

Use `camelCase` for cached references, especially optimization-layer aliases for
library functions:

```luau
const mathAbs = math.abs
const mathMin = math.min
const tableClear = table.clear
const stringFormat = string.format
```

Do not make every temporary immutable local a `const`. Use `const` for values
that communicate policy, identity, fixed behavior, or intentional cached
references.

Many rules belong to more than one concept. Cached function references are both
a variable/constant style concern and an optimization concern. This document owns
their Luau naming shape; a performance or runtime document may explain why the
cache exists.

Do not use `_` prefixes for private fields, locals, or helpers. A bare `_` is
only for intentionally ignored values.

Do not use numeric separators such as `10_000`.

Avoid new generic `State` and `States` names for modules, variables, and runtime
concepts. Name the actual state domain.

## Quick Reference

```luau
local activeConnections = {}
local function resolveTargetName(text: string): string?
end

local TextRenderer = require("./TextRenderer")
local Players = game:GetService("Players")

type RenderOptions = {
	text: string,
}

const MAX_GLYPH_COUNT = 2048
const mathMin = math.min
```
