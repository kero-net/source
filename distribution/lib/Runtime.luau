-- distribution/lib/Runtime.luau
--!strict

--# Modules

local fs = require("@lune/fs")
local process = require("@lune/process")
local Display = require("./terminal/display")

--# State

local runLogPath: string? = nil
export type JobState = Display.JobState
local display: Display.Controller? = nil

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

function Runtime.Job(name: string, state: JobState, detail: string?)
	Display.Update(assert(display, "display has not started"), name, state, detail)
	recordRunLog(string.format("[%s] %s%s", state, name, if detail then ": " .. detail else ""))
end

function Runtime.Confirm(message: string): boolean
	return Display.Confirm(display, message)
end

function Runtime.Finish(message: string)
	Display.Message(message)
	recordRunLog(message)
end

function Runtime.StartRunLog(path: string)
	runLogPath = path
	fs.writeFile(path, "KERO local build run log\n\n")
end

function Runtime.Run(root: string, line: string, reportFailure: boolean?, scope: string?): (boolean, string)
	local name = (scope or "controller"):gsub("[^%w%-_.]", "-")
	local stageLog = root .. "/.heap/logs/" .. name .. ".log"
	if not fs.isFile(stageLog) then
		fs.writeFile(stageLog, "KERO stage log: " .. name .. "\n\n")
	end
	recordRunLog("+ " .. line)
	if process.os ~= "windows" and runLogPath then
		local wrapped = "set -o pipefail; cd " .. shellQuote(root)
			.. "; { " .. line .. "; } 2>&1 | tee -a " .. shellQuote(stageLog)
			.. " >> " .. shellQuote(runLogPath)
		local result = process.exec("/bin/bash", { "--noprofile", "--norc", "-lc", wrapped })
		if result.ok then
			recordRunLog("[ok] exit 0")
			return true, ""
		end
		local message = "command exited with code " .. tostring(result.code) .. "; see .heap/logs/" .. name .. ".log"
		recordRunLog("[error] " .. message)
		return false, message
	end

	local shell = Runtime.IsWindows() and (process.env.KERO_POSIX_SHELL or "C:/Program Files/Git/bin/sh.exe") or "sh"
	local result = process.exec(shell, { "-lc", "cd " .. Runtime.Quote(root) .. " && " .. line })

	if result.stdout ~= "" then
		recordRunLog((result.stdout:gsub("%s+$", "")))
	end
	if result.stderr ~= "" then
		recordRunLog((result.stderr:gsub("%s+$", "")))
	end

	if result.ok then
		recordRunLog("[ok] exit 0")
		return true, ""
	end

	local message = result.stderr ~= "" and result.stderr or result.stdout
	recordRunLog("[error] " .. message:gsub("%s+$", ""))
	if reportFailure ~= false then
		recordRunLog("[error] Command failed; see .heap/logs/validate-locally.log for its complete output.")
	end
	return false, message
end

function Runtime.Capture(root: string, line: string): (string?, string?)
	local shell = Runtime.IsWindows() and (process.env.KERO_POSIX_SHELL or "C:/Program Files/Git/bin/sh.exe") or "sh"
	local result = process.exec(shell, { "-lc", "cd " .. Runtime.Quote(root) .. " && " .. line })
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
