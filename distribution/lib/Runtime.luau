-- distribution/lib/Runtime.luau
--!strict

--# Modules

local fs = require("@lune/fs")
local process = require("@lune/process")
local Display = require("./terminal/display")

--# State

local runLogPath: string? = nil
local environmentOverrides: { [string]: string } = {}
export type JobState = Display.JobState
local display: Display.Controller? = nil
local stepStarted: { [string]: { started: number, label: string } } = {}

local function wallMilliseconds(): number
	local result = process.exec("date", { "+%s%3N" })
	return tonumber(result.stdout) or 0
end

--# Helpers

local function shellQuote(value: string): string
	return "'" .. value:gsub("'", "'\\''") .. "'"
end

local function recordRunLog(entry: string)
	if runLogPath then
		if process.os == "windows" then
			local current = if fs.isFile(runLogPath) then fs.readFile(runLogPath) else ""
			fs.writeFile(runLogPath, current .. entry .. "\n")
		else
			process.exec("sh", { "-c", "printf '%s\\n' " .. shellQuote(entry) .. " >> " .. shellQuote(runLogPath) })
		end
	end
end

--# Functions

local Runtime = {}

function Runtime.IsWindows(): boolean
	return process.os == "windows"
end

function Runtime.IsWsl(): boolean
	return process.os == "linux" and process.env.WSL_INTEROP ~= nil
end

function Runtime.Path(root: string, child: string): string
	return root .. "/" .. child
end

function Runtime.Quote(value: string): string
	return shellQuote(value)
end

function Runtime.ReadFile(path: string): string
	Runtime.Status("Reading", path)
	return fs.readFile(path)
end

function Runtime.WriteFile(path: string, contents: string)
	fs.writeFile(path, contents)
end

function Runtime.AtomicWrite(path: string, contents: string)
	local temporary = path .. ".temporary"
	fs.writeFile(temporary, contents)
	fs.move(temporary, path, true)
end

function Runtime.WriteDirectory(path: string)
	if not fs.isDir(path) then
		fs.writeDir(path)
	end
end

function Runtime.IsFile(path: string): boolean
	return fs.isFile(path)
end

function Runtime.IsDirectory(path: string): boolean
	return fs.isDir(path)
end

function Runtime.ReadDirectory(path: string): { string }
	Runtime.Status("Listing", path)
	return fs.readDir(path)
end

function Runtime.RemoveDirectory(path: string)
	if fs.isDir(path) then
		fs.removeDir(path)
	end
end

function Runtime.Copy(from: string, to: string)
	fs.copy(from, to, true)
end

function Runtime.Move(from: string, to: string)
	fs.move(from, to, true)
end

function Runtime.Print(message: string)
	recordRunLog(message)
end

function Runtime.Log(message: string)
	recordRunLog(message)
end

function Runtime.StartDisplay(names: { string })
	display = Display.Create(names)
end

function Runtime.Job(name: string, state: JobState, detail: string?, total: number?)
	if display then
		Display.Update(display, name, state, detail, total)
	else
		print(string.format("[%s] %s%s", state, name, if detail then ": " .. detail else ""))
	end
	recordRunLog(string.format("[%s] %s%s", state, name, if detail then ": " .. detail else ""))
end

function Runtime.Step(name: string, action: string, message: string)
	if display then
		Display.Step(display, name, action, message)
	else
		print(string.format("[%s] %s: %s", action:lower(), name, message))
	end
	recordRunLog(string.format("[%s] %s: %s", action:lower(), name, message))
	stepStarted[name] = { started = wallMilliseconds(), label = message }
end

function Runtime.StepComplete(name: string)
	if display then
		Display.StepComplete(display, name)
	end
	local step = stepStarted[name]
	if step then
		local elapsed = wallMilliseconds() - step.started
		if elapsed >= 0 then
			local line = string.format("[timing] %s: %s %.2fs", name, step.label, elapsed / 1000)
			recordRunLog(line)
			if display then
				Display.Status(display, "Timing", line)
			else
				print(line)
			end
		end
		stepStarted[name] = nil
	end
end

function Runtime.Status(action: string, message: string, dim: boolean?)
	if display then
		Display.Status(display, action, message, dim)
	else
		print(string.format("[%s] %s", action:lower(), message))
	end
	recordRunLog(string.format("[%s] %s", action:lower(), message))
end

function Runtime.Confirm(message: string): boolean
	return Display.Confirm(display, message)
end

function Runtime.Finish(message: string)
	if display then
		Display.Message(display, message)
	else
		print(message)
	end
	recordRunLog(message)
end

function Runtime.StartRunLog(path: string)
	runLogPath = path
	local previous = if fs.isFile(path) then fs.readFile(path) else ""
	fs.writeFile(path, previous .. (if previous == "" then "KERO local build run log\n\n" else "\nKERO validation retry\n\n"))
end

function Runtime.Run(root: string, line: string, reportFailure: boolean?, scope: string?): (boolean, string)
	local name = (scope or "controller"):gsub("[^%w%-_.]", "-")
	Runtime.WriteDirectory(root .. "/.heap")
	Runtime.WriteDirectory(root .. "/.heap/logs")
	local stageLog = root .. "/.heap/logs/" .. name .. ".log"
	if not fs.isFile(stageLog) then
		fs.writeFile(stageLog, "KERO stage log: " .. name .. "\n\n")
	end
	Runtime.Status("Running", name .. ": " .. line)
	recordRunLog("+ " .. line)
	local wrapped = "set -o pipefail; cd " .. shellQuote(root)
		.. " && { " .. line .. "; } 2>&1 | tee -a " .. shellQuote(stageLog)
	if runLogPath then
		wrapped ..= " " .. shellQuote(runLogPath)
	end
	local shell = Runtime.IsWindows() and (process.env.KERO_POSIX_SHELL or "C:/Program Files/Git/bin/sh.exe") or "/bin/bash"
	local child = process.create(shell, { "--noprofile", "--norc", "-lc", wrapped }, { env = environmentOverrides })
	child.stdin:close()
	while true do
		local chunk = child.stdout:read()
		if chunk == nil then
			break
		end
		if display then
			Display.Output(display, chunk)
		else
			print(chunk)
		end
	end
	local result = child:status()
	if result.ok then
		recordRunLog("[ok] exit 0")
		return true, ""
	end
	local message = "command exited with code " .. tostring(result.code) .. "; see .heap/logs/" .. name .. ".log"
	recordRunLog("[error] " .. message)
	if reportFailure ~= false then
		recordRunLog("[error] Command failed; see .heap/logs/validate-locally.log for its complete output.")
	end
	return false, message
end

function Runtime.Capture(root: string, line: string): (string?, string?)
	local shell = Runtime.IsWindows() and (process.env.KERO_POSIX_SHELL or "C:/Program Files/Git/bin/sh.exe") or "sh"
	Runtime.Status("Inspecting", line)
	local result = process.exec(shell, { "-lc", "cd " .. Runtime.Quote(root) .. " && " .. line }, { env = environmentOverrides })
	recordRunLog("+ " .. line)
	if result.stdout ~= "" then
		recordRunLog((result.stdout:gsub("%s+$", "")))
	end
	if result.stderr ~= "" then
		recordRunLog((result.stderr:gsub("%s+$", "")))
	end
	if result.ok then
		recordRunLog("[ok] exit 0")
		return result.stdout:gsub("%s+$", ""), nil
	end

	local message = result.stderr ~= "" and result.stderr or result.stdout
	recordRunLog("[error] " .. message:gsub("%s+$", ""))
	return nil, message
end

function Runtime.Environment(name: string): string?
	return process.env[name]
end

function Runtime.SetEnvironment(name: string, value: string)
	process.env[name] = value
	environmentOverrides[name] = value
end

function Runtime.CurrentDirectory(): string
	return process.cwd:gsub("\\", "/")
end

function Runtime.OperatingSystem(): string
	return process.os
end

function Runtime.Architecture(): string
	return process.arch
end

return Runtime
