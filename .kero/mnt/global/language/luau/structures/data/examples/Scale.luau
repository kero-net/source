-- structures/data: Scale.luau
--!strict

const base = 8

return {
	base = base,

	step = function(index: number): number
		return index * base
	end,
}
