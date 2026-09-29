# Luau Files

This document owns Luau file shape, role headers, module-head shape, comments,
and style variation by module kind.

The user's Luau style is global, but it is not one rigid skeleton. Different
module kinds may intentionally use rotated versions of the style. Treat that as
contextual style, not automatic inconsistency.

## File Shape

Use the path label and `--!strict` at the top of every Luau source file.

Use role headers only when they improve scanning. A tiny file does not need decorative structure.

Preferred phase order:

1. path label/directives
2. module identity
3. dependencies
4. types/constants/variables
5. helpers
6. public surface
7. initialization
8. return

## Module Trees

Do not create `init.luau` just because a folder exists. Use it when the folder
is the runtime, tool, app, or subsystem head.

A good `init.luau` owns at least one of these:

- the public contract for the runtime, tool, app, or subsystem
- validation or normalization at the boundary
- lifecycle and setup/teardown
- coordination between private child modules
- a stable facade used by callers or by a higher subsystem head

It should not be a mystery table of child exports. If callers still need to
know and require every child module under it, the folder head probably is not
doing real work.

Prefer this shape for a subsystem:

```text
Render/
  init.luau      # owns RenderText/MeasureText/CreateAtlas for the runtime head
  Text.luau      # private implementation detail
  Layout.luau    # private implementation detail
  Raster.luau    # private implementation detail
```

Avoid placeholder folder heads:

```text
Serialize/
  init.luau      # bad if it only returns child modules or empty structure
```

Use a direct module until the concept earns a folder:

```text
Serialize.luau
```

## Basic Module Template

```luau
-- src/app/Render/Text.luau
--!strict

--# Modules

local Settings = require("../Settings")

--# Types

export type RenderTextOptions = {
	Text: string,
}

--# Constants

const DEFAULT_TEXT = ""

--# Helpers

local function normalizeText(text: string?): string
	return text or DEFAULT_TEXT
end

--# Functions

local Text = {}

function Text.Render(options: RenderTextOptions)
	local text = normalizeText(options.Text)
	-- render behavior
end

return Text
```

Tiny modules can omit role headers when the structure is obvious.

## Headers

Allowed common headers:

```luau
--# Services
--# Modules
--# Types
--# Constants
--# Variables
--# Helpers
--# Constructor
--# Functions
--# Methods
--# Lifecycle
--# Initialization
```

Avoid `--# Imports`, `--# Module`, `--# Public Methods`, and `--# Private Methods`.

Private behavior should usually be local helpers, not table-private methods.

## Comments

Comments should explain ownership, pressure points, non-obvious behavior,
validation boundaries, lifecycle assumptions, runtime cost, nil cases, and caller
obligations.

Do not write hollow comments that restate the function name.

Do not block-document obvious constructors or one-line methods. Add docs when
there is ownership, lifetime, side effect, nil behavior, ordering, allocation,
networking, or caller responsibility that is not obvious from the signature.

Data-only modules should keep documentation close to the fields it explains, but
do not document every leaf equally.
