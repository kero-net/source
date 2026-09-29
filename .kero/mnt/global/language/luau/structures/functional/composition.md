# Functional Composition

Prefer named helpers when composition would otherwise hide validation,
allocation, nil behavior, or mutation.

Keep dense expression chains out of boundary code.

Functional composition is useful when the intermediate names would not add
meaning. It gets brittle when each step has different failure behavior or side
effects.

Use helpers to expose the why:

```luau
local function normalizeRatio(value: number): number
	return math.clamp(value, 0, 1)
end

return function(value: number, scale: number): number
	return normalizeRatio(value) * scale
end
```

Avoid compressing validation-heavy code into a clever chain. If readers need to
pause and reconstruct the failure cases, the composition is hiding ownership.
