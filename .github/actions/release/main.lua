#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/actions/release/main%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local releases = require("releases.validate")

local version = arg[1] or os.getenv("VERSION")
local ok, message = releases.validate(root, version)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

local tags, tag_error = command.capture(root, "git tag --list 'v*'")
if not tags then io.stderr:write(tag_error .. "\n"); os.exit(1) end
ok, message = releases.validate_sequence(version, tags)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

local tag = "v" .. version
local output = os.getenv("GITHUB_OUTPUT")
if output and output ~= "" then
  local file, output_error = io.open(output, "a")
  if not file then io.stderr:write(output_error .. "\n"); os.exit(1) end
  file:write("tag=", tag, "\n")
  file:close()
end
io.stdout:write("Validated release ", version, "\n")
