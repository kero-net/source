# ECS Queries

Queries represent entity/component selection.

Query style is not finalized yet. Define concrete query shape from the first
real ECS implementation.

Why this shape exists:

- queries express which component sets a system cares about
- selection logic stays separate from the behavior that consumes selected data
- shared queries can make repeated system filters auditable

Sketch only:

```luau
local movingTextTargets = {
	"TextTarget",
	"Position",
	"Velocity",
}

return movingTextTargets
```

This is not a final query representation. Real ECS source should decide whether
queries are tables, functions, builder calls, cached handles, or framework-owned
objects.
