-- distribution/lib/terminal/progress.luau
--!strict

--# Functions

local Progress = {}

function Progress.Segments(completed: number, total: number, width: number): (string, string)
	assert(total > 0, "progress requires at least one action")
	assert(completed >= 0 and completed <= total, "progress completion is out of range")
	assert(width > 0, "progress width must be positive")

	if completed == total then
		return string.rep("=", width), ""
	end

	local filled = math.floor((completed * width) / total)
	if filled >= width then
		filled = width - 1
	end

	local active = string.rep("=", filled) .. ">"
	return active, string.rep(" ", width - #active)
end

function Progress.Count(completed: number, total: number): string
	assert(total > 0, "progress requires at least one action")
	assert(completed >= 0 and completed <= total, "progress completion is out of range")
	return string.format("%d/%d", completed, total)
end

return Progress
