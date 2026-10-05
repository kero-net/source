--!nonstrict
local runtime = require("../actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg

local command = require("../actions/lib/command")
local releases = require("../../releases/validate")

local version = arg[1] or os.getenv("VERSION")
local ok, message = releases.validate(".", version)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

local tags, tag_error = command.capture(".", "git tag --list 'v*'")
if not tags then io.stderr:write(tag_error .. "\n"); os.exit(1) end
ok, message = releases.validate_sequence(version, tags)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

local output = os.getenv("GITHUB_OUTPUT")
if output and output ~= "" then
  local file, output_error = io.open(output, "a")
  if not file then io.stderr:write(output_error .. "\n"); os.exit(1) end
  file:write("tag=v", version, "\n")
  file:close()
end
io.stdout:write("Validated release ", version, "\n")
