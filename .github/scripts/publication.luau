--!nonstrict
local runtime = require("../actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local script = (".github/scripts/publication.luau"):gsub("\\", "/")
local root = script:match("^(.*)/%.github/scripts/publication%.luau$") or "."

local releases = require("../../releases/validate")
local file = assert(io.open(root .. "/releases/publication.json", "rb"))
local value = file:read("*a")
file:close()

local enabled = value:match('"enabled"%s*:%s*(%a+)')
local record = value:match('"record"%s*:%s*"([^"]+)"')
local tag = value:match('"tag"%s*:%s*"([^"]+)"')
if (enabled ~= "true" and enabled ~= "false") or not record or not tag then
  error("invalid releases/publication.json")
end
if not releases.validate(root, record) then error("publication record is not authored: " .. record) end
if tag ~= "v" .. record then error("publication tag must be v plus its record") end

local output = os.getenv("GITHUB_OUTPUT")
if output then
  local stream = assert(io.open(output, "a"))
  stream:write("enabled=", enabled, "\nrecord=", record, "\ntag=", tag, "\n")
  stream:close()
end
io.stdout:write("Publication ", enabled, " for ", record, " (", tag, ")\n")
