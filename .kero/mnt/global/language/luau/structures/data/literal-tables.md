# Literal Data Tables

Prefer a direct returned table when the module does not need setup:

```luau
-- src/app/Theme/Tokens.luau
--!strict

return {
	spacing = {
		small = 4,
		medium = 8,
		large = 12,
	},

	color = {
		text = Color3.fromRGB(235, 235, 235),
		panel = Color3.fromRGB(24, 26, 30),
	},
}
```

Keep the table readable as a value.

Avoid adding a local module table only to immediately return it:

```luau
local Tokens = {}

Tokens.spacing = {
	small = 4,
}

return Tokens
```

That shape implies procedural construction. If the module is just data, return
the data.

## Function Values

Function values are allowed when they are part of the declared shape:

```luau
-- src/app/Theme/Scale.luau
--!strict

const base = 8

return {
	base = base,

	step = function(index: number): number
		return index * base
	end,
}
```

If functions become the main point of the module, use a procedural table module
or function module instead.
