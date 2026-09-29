-- distribution/scripts/build/build-every-distribution.luau
--!strict

--# Modules

local process = require("@lune/process")
local Runtime = require("../../lib/Runtime")
local Complete = require("../../lib/build/complete")

--# Initialization

assert(#process.args == 0, "build-every-distribution accepts no arguments")
local root = process.cwd:gsub("\\", "/")
local succeeded, failure = pcall(Complete.Run, root)
if not succeeded then
	Runtime.Log("[fatal] " .. tostring(failure))
	Runtime.Finish("[error] Local validation is incomplete; successful artifacts were preserved. See .heap/RELEASE.md and .heap/logs/validate-locally.log.")
	process.exit(1)
end
