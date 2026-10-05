# ECS Systems

Systems represent behavior over queried entities.

System style is not finalized yet. Define concrete system shape from the first
real ECS implementation.

Why this shape exists:

- behavior is applied across a selected set of entities
- the system should depend on component data, not per-entity methods
- iteration order, mutation rules, and scheduling become explicit concerns

Sketch only:

```luau
type Entity = number
type Position = {
	x: number,
	y: number,
}
type Velocity = {
	x: number,
	y: number,
}
type World = {
	query: (self: World, ...string) -> () -> (Entity?, Position?, Velocity?),
}

return function(world: World, deltaTime: number)
	for _entity, position, velocity in world:query("Position", "Velocity") do
		if position == nil or velocity == nil then
			continue
		end

		position.x += velocity.x * deltaTime
		position.y += velocity.y * deltaTime
	end
end
```

This demonstrates the responsibility boundary only. It does not choose a world
API, query API, scheduler, or mutation policy for adopting repositories.
