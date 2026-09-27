#!/usr/bin/env lua
-- Builds, verifies, optionally GPG-signs, and optionally launches one native Qt release.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/distribution/scripts/package%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path
local command = require("lib.command")
local settings = require("distribution.lib.settings")
local lua = os.getenv("KERO_LUA") or "lua"
local cargo = package.config:sub(1, 1) == "\\" and "cargo +stable-aarch64-pc-windows-gnullvm" or "cargo"

local function host_target()
  if package.config:sub(1, 1) == "\\" then
    local probe = io.popen('powershell.exe -NoProfile -NonInteractive -Command "(Get-CimInstance -ClassName Win32_Processor | Select-Object -First 1 -ExpandProperty Architecture)"')
    local architecture = probe and (probe:read("*l") or "") or ""
    if probe then probe:close() end
    return architecture == "12" and "windows-arm64" or "windows-x64"
  end
  local pipe = assert(io.popen("uname -s 2>/dev/null"))
  local name = pipe:read("*l") or ""
  pipe:close()
  return name == "Darwin" and "macos-arm64" or "linux-x64"
end

local target, launch = os.getenv("KERO_RELEASE_TARGET") or host_target(), false
for _, value in ipairs(arg) do
  if value == "--launch" then launch = true else target = value end
end
if target == "current" then target = host_target() end
if target ~= "windows-arm64" and target ~= "windows-x64" and target ~= "linux-x64" and target ~= "macos-arm64" then
  error("target must be current, windows-arm64, windows-x64, linux-x64, or macos-arm64")
end

local function distribution_target(name)
  local policy = settings.target(root, name)
  if not policy then return nil end
  return {
    artifact = policy.artifact, required = policy.required_environment,
    enabled = policy.enabled, lifecycle = policy.lifecycle,
  }
end

local policy = distribution_target(target)
if not policy then error("target is not enabled by distribution/distribution.kst: " .. target) end
if not policy.enabled then error("target is " .. policy.lifecycle .. "; native macOS packaging requires a Mac contributor toolchain") end

local function require_environment(names)
  local missing = {}
  for _, name in ipairs(names) do
    if not os.getenv(name) or os.getenv(name) == "" then missing[#missing + 1] = name end
  end
  if #missing > 0 then
    error("Package target " .. target .. " requires: " .. table.concat(missing, ", ")
      .. ". Configure the matching native Qt/toolchain environment described in distribution/README.md.")
  end
end

local required = {}
for name in policy.required:gmatch("[^,]+") do required[#required + 1] = name end
require_environment(required)
local qt_prefix = os.getenv("KERO_QT_PREFIX")
local deploy_tool = os.getenv("KERO_DEPLOY_TOOL")
if not deploy_tool or deploy_tool == "" then
  local executable = target:match("^windows") and "windeployqt.exe" or (target == "macos-arm64" and "macdeployqt" or "linuxdeployqt")
  deploy_tool = qt_prefix .. "/bin/" .. executable
end

local build = ".heap/build/qt/" .. target
local release = ".heap/build/releases/" .. target
local cargo_target = ".heap/build/cargo/target"
local function working_directory()
  local command_name = package.config:sub(1, 1) == "\\" and "cd" or "pwd"
  local directory = assert(io.popen(command_name))
  local value = directory:read("*l") or root
  directory:close()
  return value:gsub("\\", "/")
end
local workspace = working_directory()
local wasm = workspace .. "/" .. cargo_target .. "/wasm32-wasip1/release/kero_core.wasm"
local host_extension = target:match("^windows") and ".exe" or ""
local host = workspace .. "/" .. cargo_target .. "/release/kero-host" .. host_extension
local function run(value)
  io.stdout:write("==> ", value, "\n")
  local ok, message = command.run(root, value, true)
  if not ok then error(message) end
end
run(lua .. " distribution/scripts/validate-toolchain.lua " .. command.quote(target))
run(lua .. " distribution/scripts/validate-key.lua")
local gpg_key = os.getenv("KERO_SIGN_RELEASE") == "1" and os.getenv("KERO_GPG_KEY_ID") or nil

local function verify_artifact(artifact)
  run("cmake -E sha256sum " .. command.quote(artifact) .. " > " .. command.quote(artifact .. ".sha256"))
  if gpg_key and gpg_key ~= "" then
    run("gpg --batch --local-user " .. command.quote(gpg_key) .. " --detach-sign --armor " .. command.quote(artifact))
    run("gpg --verify " .. command.quote(artifact .. ".asc") .. " " .. command.quote(artifact))
  end
end

if target:match("^windows") then
  local png = root .. "/assets/images/kero-icon.png"
  local icon = workspace .. "/.heap/build/qt/" .. target .. "/kero-icon.ico"
  -- A Windows ARM64 build must never reuse an x64 cross-build cache: CMake
  -- otherwise retains LLVM-MinGW's archiver even after the MSVC compiler wins.
  if target == "windows-arm64" then
    run("cmake -E rm -rf " .. command.quote(build))
  end
  run("cmake -E make_directory " .. command.quote(build))
  local input = assert(io.open(png, "rb")):read("*a")
  local output = assert(io.open(icon, "wb"))
  output:write(string.char(0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 32, 0))
  local size, offset = #input, 22
  output:write(string.char(size % 256, math.floor(size / 256) % 256, math.floor(size / 65536) % 256, math.floor(size / 16777216) % 256))
  output:write(string.char(offset, 0, 0, 0), input)
  output:close()
  -- KERO_ICON is deliberately generated in the ignored build tree.
  run(cargo .. " build --manifest-path src/Cargo.toml --release -p kero-core --target wasm32-wasip1")
  run(cargo .. " build --manifest-path src/Cargo.toml --release -p kero-cli")
  local configure_env = "KERO_ICON=" .. command.quote(icon) .. " KERO_WASM=" .. command.quote(wasm) .. " KERO_HOST=" .. command.quote(host) .. " KERO_DEPLOY_TOOL=" .. command.quote(deploy_tool)
  local msvc_tools = ""
  if target == "windows-arm64" then
    local linker = assert(os.getenv("KERO_MSVC_LINKER"), "KERO_MSVC_LINKER must name the Visual Studio ARM64 linker")
    local archiver = assert(os.getenv("KERO_MSVC_ARCHIVER"), "KERO_MSVC_ARCHIVER must name the Visual Studio ARM64 librarian")
    msvc_tools = " -DCMAKE_LINKER:FILEPATH=" .. command.quote(linker)
      .. " -DCMAKE_AR:FILEPATH=" .. command.quote(archiver)
  end
  run(configure_env .. " cmake --preset " .. target .. " -S host/qt -B " .. command.quote(build) .. msvc_tools)
else
  run(cargo .. " build --manifest-path src/Cargo.toml --release -p kero-core --target wasm32-wasip1")
  run(cargo .. " build --manifest-path src/Cargo.toml --release -p kero-cli")
  run("KERO_WASM=" .. command.quote(wasm) .. " KERO_HOST=" .. command.quote(host) .. " KERO_DEPLOY_TOOL=" .. command.quote(deploy_tool) .. " cmake --preset " .. target .. " -S host/qt -B " .. command.quote(build))
end
run("cmake --build " .. command.quote(build) .. " --target kero-package")

if target:match("^windows") then
  run("cmake --build " .. command.quote(build) .. " --target kero-windows-installer")
  local artifact = release .. "/kero-installer.exe"
  verify_artifact(artifact)
elseif target == "linux-x64" then
  run("cmake --build " .. command.quote(build) .. " --target kero-linux-appimage")
  local artifact = release .. "/kero.AppImage"
  verify_artifact(artifact)
else
  local dmg, find_error = command.capture(root, "find " .. command.quote(release) .. " -maxdepth 1 -name '*.dmg' -print -quit")
  if not dmg or dmg == "" then error("macOS package did not produce a DMG: " .. tostring(find_error or release)) end
  verify_artifact(dmg)
end

if launch then
  local installer = target:match("^windows") and release .. "/kero-installer.exe" or release .. "/kero-install"
  run(command.quote(installer))
end
