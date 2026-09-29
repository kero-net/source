--!strict

-- Analyzer-only declaration for Lune's cooperative task scheduler.
local Task = {} :: {
	spawn: (callback: () -> ()) -> thread,
	wait: (duration: number?) -> number,
}

return Task
