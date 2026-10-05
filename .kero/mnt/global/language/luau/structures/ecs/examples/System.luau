-- structures/ecs: System.luau
--!strict

-- Sketch only. Final ECS shape should come from real ECS source.
-- Shows the boundary: a system iterates matching entities and applies behavior.

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
