-- distribution/lib/terminal/progress.luau
--!strict

-- The bounded fill calculation follows the presentation model used by
-- pacman's fill_progress function, without carrying over its terminal I/O,
-- animation, colors, callbacks, or package-manager state.
-- Source: https://gist.github.com/TaylanTatli/e813c2560b0f1ecc9f5d

local Progress = {}

function Progress.Format(completed: number, total: number, width: number?): string
	assert(total > 0, "progress requires at least one action")
	assert(completed >= 0 and completed <= total, "progress completion is out of range")
	local barWidth = width or 28
	local percent = math.floor((completed * 100) / total)
	local filled = math.floor((percent * barWidth) / 100)
	local bar = string.rep("█", filled) .. string.rep(" ", barWidth - filled)
	return string.format("   Building [%s] %3d%% (%d/%d)", bar, percent, completed, total)
end

return Progress
