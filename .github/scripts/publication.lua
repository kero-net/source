#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/scripts/publication%.lua$") or "."
package.path = root .. "/?.lua;" .. package.path

local releases = require("releases.validate")
local file = assert(io.open(root .. "/releases/publication.json", "rb"))
local value = file:read("*a")
file:close()

local enabled = value:match('"enabled"%s*:%s*(true|false)')
local record = value:match('"record"%s*:%s*"([^"]+)"')
local tag = value:match('"tag"%s*:%s*"([^"]+)"')
if not enabled or not record or not tag then error("invalid releases/publication.json") end
if not releases.validate(root, record) then error("publication record is not authored: " .. record) end
if tag ~= "v" .. record then error("publication tag must be v plus its record") end

local output = os.getenv("GITHUB_OUTPUT")
if output then
  local stream = assert(io.open(output, "a"))
  stream:write("enabled=", enabled, "\nrecord=", record, "\ntag=", tag, "\n")
  stream:close()
end
io.stdout:write("Publication ", enabled, " for ", record, " (", tag, ")\n")
