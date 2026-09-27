#!/usr/bin/env lua
-- Checks the non-secret package toolchain contract before a build begins.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/distribution/scripts/validate%-toolchain%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path
local command = require("lib.command")
local settings = require("distribution.lib.settings")
local lua = os.getenv("KERO_LUA") or "lua"

local target = arg[1] or os.getenv("KERO_RELEASE_TARGET")
if not target or target == "" then error("release target is required") end
local policy = settings.target(root, target)
if not policy then error("unknown distribution target: " .. target) end
for name in policy.required_environment:gmatch("[^,]+") do
  local value = os.getenv(name)
  if not value or value == "" then error(target .. " requires " .. name) end
end
for _, tool in ipairs({ { "cmake", "--version" }, { "cargo", "--version" }, { lua, "-v" } }) do
  local ok = command.run(root, tool[1] .. " " .. tool[2], true)
  if not ok then error(target .. " requires executable: " .. tool[1]) end
end
local installed, details = command.capture(root, "rustup target list --installed")
if not installed or not installed:find("wasm32%-wasip1") then
  error(target .. " requires Rust target wasm32-wasip1; run: rustup target add wasm32-wasip1\n" .. tostring(details or ""))
end
if target == "windows-arm64" then
  local compiler = command.capture(root, "where.exe cl.exe")
  if not compiler or compiler == "" then
    error("windows-arm64 requires the MSVC ARM64 compiler. Install the Visual Studio Build Tools C++ ARM64 component, then run from its ARM64 Developer Command Prompt.")
  end
end
io.stdout:write("[ok] Toolchain contract is present for ", target, ".\n")
