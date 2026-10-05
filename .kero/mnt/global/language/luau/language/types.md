# Luau Types

This document owns type-safety constraints and allowed uncertainty boundaries.

## Main Rule

A cast is not a type-safety fix. Treat casts as evidence that the type surface, loader shape, API boundary, or ownership model may be wrong.

When a type error appears, resolve it in this order:

1. Improve inference.
2. Improve the API or helper type.
3. Improve the typed lookup surface.
4. Improve the caller's type.
5. Add an explicit structural contract.
6. Move validation to the actual boundary.
7. Use a named boundary pattern only if the uncertainty truly belongs there.

Never fix a type error by casting to `any`. "The analyzer does not understand this" means the design needs sharper types, not a shrug-cast.

## Allowed Boundary Patterns

Allowed boundary patterns are rare.

Constructor-boundary casts are allowed only when staged metatable construction must pass incomplete `self` through helpers/callbacks typed as the final instance.

Raw input quarantine casts are allowed only at external boundaries before validation.

Engine/API boundary casts are allowed only when Luau cannot currently express the engine shape and the uncertainty is localized.

## Lookup Types

Prefer typed lookup surfaces for owned child sets:

```luau
local Settings = {
	Render = require("./Settings/Render"),
	Import = require("./Settings/Import"),
}

export type SettingGroup = keyof<typeof(Settings)>
export type SettingsMap = typeof(Settings)

return Settings
```

If a manifest is unavoidable, preserve exact table shape before extracting ids. Do not widen first:

```luau
local Actions: { [string]: ActionDefinition } = { ... }
```

That destroys the literal key union.

## Values Utility

For a known table shape, a value union is:

```luau
type Values<T> = index<T, keyof<T>>
```

For metatable-sensitive cases, consider `rawkeyof` and `rawget` when inherited `__index` keys should not count.

## Unknown vs Any

Use `unknown` for raw values that must be validated.

Avoid `any`. If it appears necessary, first try a typed lookup surface, explicit
structural contract, validated boundary, or API redesign. Do not export `any`.
