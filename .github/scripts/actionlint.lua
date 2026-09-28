#!/usr/bin/env lua
package.path = "./.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local filesystem = require("lib.filesystem")

local version = "1.7.12"
local is_windows = package.config:sub(1, 1) == "\\"
local machine = is_windows and "windows" or assert(io.popen("uname -m", "r")):read("*l")
local assets = {
  x86_64 = {
    archive = "actionlint_" .. version .. "_linux_amd64.tar.gz",
    checksum = "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8",
  },
  aarch64 = {
    archive = "actionlint_" .. version .. "_linux_arm64.tar.gz",
    checksum = "325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6",
  },
}
local asset = assets[machine]
local tool_root = ".heap/tools/actionlint"
local binary = tool_root .. "/actionlint"

if is_windows then
  local installed = command.capture(".", "where.exe actionlint.exe")
  if installed and installed ~= "" then binary = installed:match("[^\r\n]+") end
  if not binary:match("%.exe$") then
    local winget = (os.getenv("LOCALAPPDATA") or "")
      .. "/Microsoft/WinGet/Packages/rhysd.actionlint_Microsoft.Winget.Source_8wekyb3d8bbwe/actionlint.exe"
    local file = io.open(winget, "rb")
    if file then file:close(); binary = winget end
  end
end

local supplied = os.getenv("ACTIONLINT_BIN")
if supplied and supplied ~= "" then
  local ok, message = command.run(".", command.quote(supplied) .. " -color")
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
  os.exit(0)
end

if is_windows and binary:match("%.exe$") then
  local ok, message = command.run(".", command.quote(binary) .. " -color")
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
  os.exit(0)
end

if not asset then
  io.stderr:write("Unsupported actionlint host architecture: " .. tostring(machine) .. "\n")
  io.stderr:write("Set ACTIONLINT_BIN to a compatible actionlint executable.\n")
  os.exit(1)
end

local archive = tool_root .. "/" .. asset.archive

local ok, message = command.run(".", "mkdir -p " .. command.quote(tool_root), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

if not command.run(".", "test -x " .. command.quote(binary), true) then
  local url = "https://github.com/rhysd/actionlint/releases/download/v" .. version .. "/" .. asset.archive
  ok, message = command.run(".",
    "curl --fail --location --silent --show-error " .. command.quote(url) .. " --output " .. command.quote(archive))
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

  local checksum = tool_root .. "/actionlint.sha256"
  local wrote, write_error = filesystem.write("./" .. checksum, asset.checksum .. "  " .. archive .. "\n")
  if not wrote then io.stderr:write(write_error .. "\n"); os.exit(1) end

  ok, message = command.run(".", "sha256sum --check --strict " .. command.quote(checksum))
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
  ok, message = command.run(".",
    "tar -xzf " .. command.quote(archive) .. " -C " .. command.quote(tool_root) .. " actionlint"
      .. " && chmod 700 " .. command.quote(binary))
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end

ok, message = command.run(".", command.quote(binary) .. " -color")
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
