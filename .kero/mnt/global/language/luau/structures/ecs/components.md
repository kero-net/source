# ECS Components

Components represent data attached to entities.

Component style is not finalized yet. Define concrete component shape from the
first real ECS implementation.

Why this shape exists:

- components name entity data, not behavior
- many entities can have the same component kind
- systems can query components without knowing object lifetimes

Sketch only:

```luau
export type Position = {
	x: number,
	y: number,
}
```

This is not yet a rule for component storage, naming, or registration. Those
choices need real ECS source before they become policy.
