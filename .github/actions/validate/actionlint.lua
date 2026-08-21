#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/actions/validate/actionlint%.lua$") or "."
package.path = root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local filesystem = require("lib.filesystem")

local version = "1.7.12"
local archive_name = "actionlint_" .. version .. "_linux_amd64.tar.gz"
local expected = "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"
local tool_root = "../.heap/source/tools/actionlint"
local binary = tool_root .. "/actionlint"
local archive = tool_root .. "/" .. archive_name

local supplied = os.getenv("ACTIONLINT_BIN")
if supplied and supplied ~= "" then
  local ok, message = command.run(root, "test -x " .. command.quote(supplied), true)
  if not ok then io.stderr:write("ACTIONLINT_BIN is not executable\n"); os.exit(1) end
  ok, message = command.run(root, command.quote(supplied) .. " -color")
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
  os.exit(0)
end

local ok, message = command.run(root, "mkdir -p " .. command.quote(tool_root), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

if not command.run(root, "test -x " .. command.quote(binary), true) then
  local url = "https://github.com/rhysd/actionlint/releases/download/v" .. version .. "/" .. archive_name
  ok, message = command.run(root,
    "curl --fail --location --silent --show-error " .. command.quote(url) .. " --output " .. command.quote(archive))
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

  local checksum = tool_root .. "/actionlint.sha256"
  local wrote, write_error = filesystem.write(root .. "/" .. checksum, expected .. "  " .. archive .. "\n")
  if not wrote then io.stderr:write(write_error .. "\n"); os.exit(1) end

  ok, message = command.run(root, "sha256sum --check --strict " .. command.quote(checksum))
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
  ok, message = command.run(root,
    "tar -xzf " .. command.quote(archive) .. " -C " .. command.quote(tool_root) .. " actionlint"
      .. " && chmod 700 " .. command.quote(binary))
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end

ok, message = command.run(root, command.quote(binary) .. " -color")
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
