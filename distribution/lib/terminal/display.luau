-- distribution/lib/terminal/display.luau
--!strict

local process = require("@lune/process")
local stdio = require("@lune/stdio")
local Progress = require("./progress")

export type JobState = "wait" | "run" | "done" | "fail" | "skip"
type Job = {
	name: string,
	state: JobState,
	detail: string?,
}
export type Controller = {
	interactive: boolean,
	jobs: { Job },
	index: { [string]: number },
	rendered: boolean,
}

local CLEAR_SCREEN_AND_HOME = "\27[2J\27[H"
local CURSOR_HOME = "\27[H"
local CLEAR_LINE = "\27[2K"
local CLEAR_TO_END = "\27[J"

local function completedJobs(controller: Controller): number
	local completed = 0
	for _, job in controller.jobs do
		if job.state == "done" or job.state == "fail" or job.state == "skip" then
			completed += 1
		end
	end
	return completed
end

local function lines(controller: Controller): { string }
	local output = {
		"KERO local validation",
		"",
		Progress.Format(completedJobs(controller), #controller.jobs),
		"",
	}
	for _, job in controller.jobs do
		local detail = if job.detail and job.detail ~= "" then "  " .. job.detail else ""
		table.insert(output, string.format("   [%-4s] %s%s", job.state, job.name, detail))
	end
	return output
end

local function render(controller: Controller)
	if not controller.interactive then
		return
	end
	stdio.write(if controller.rendered then CURSOR_HOME else CLEAR_SCREEN_AND_HOME)
	for _, line in lines(controller) do
		stdio.write(CLEAR_LINE .. line .. "\n")
	end
	stdio.write(CLEAR_TO_END)
	controller.rendered = true
end

local Display = {}

function Display.Create(names: { string }): Controller
	local controller: Controller = {
		interactive = process.env.KERO_INTERACTIVE == "1",
		jobs = {},
		index = {},
		rendered = false,
	}
	for index, name in names do
		table.insert(controller.jobs, { name = name, state = "wait" })
		controller.index[name] = index
	end
	if controller.interactive then
		render(controller)
	else
		stdio.write("KERO local validation\n")
	end
	return controller
end

function Display.Update(controller: Controller, name: string, state: JobState, detail: string?)
	local index = assert(controller.index[name], "unknown display job: " .. name)
	controller.jobs[index].state = state
	controller.jobs[index].detail = detail
	if controller.interactive then
		render(controller)
	else
		stdio.write(string.format("[%-4s] %s%s\n", state, name, if detail then " — " .. detail else ""))
	end
end

function Display.Confirm(controller: Controller?, message: string): boolean
	stdio.write(message .. " [Y/n] ")
	local answer = stdio.readLine():lower()
	if controller and controller.interactive then
		render(controller)
	end
	return answer == "" or answer == "y" or answer == "yes"
end

function Display.Message(message: string)
	stdio.write(message .. "\n")
end

return Display
