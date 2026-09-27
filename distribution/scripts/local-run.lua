#!/usr/bin/env lua
-- Local package orchestration. Package creation and runtime evidence are separate stages.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/distribution/scripts/local%-run%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path
local command = require("lib.command")
local settings = require("distribution.lib.settings")
local lua = os.getenv("KERO_LUA") or "lua"
local stages = { portable = true, build = true, verify = true, test = true, all = true, propose = true, status = true }
local stage, target = arg[1] or "build", arg[2] or "current"
assert(stages[stage], "usage: kero [portable|build|verify|test|all|propose|status] [current|target] [--adapter NAME]")
local adapter = "native"
for index = 3, #arg do if arg[index] == "--adapter" then adapter = assert(arg[index + 1], "--adapter requires a name") end end

local function read(path)
  local file = assert(io.open(root .. "/" .. path, "rb"), "missing " .. path)
  local text = file:read("*a"); file:close(); return text
end
local function target_list()
  local list = {}
  for _, item in pairs(settings.read(root).targets) do list[#list + 1] = item end
  table.sort(list, function(left, right) return left.name < right.name end)
  return list
end
local function policy(name) for _, item in ipairs(target_list()) do if item.name == name then return item end end end
local function host_target()
  if package.config:sub(1, 1) == "\\" then
    local pipe = io.popen('powershell.exe -NoProfile -NonInteractive -Command "(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Architecture)"')
    local code = pipe and (pipe:read("*l") or "") or ""; if pipe then pipe:close() end
    return code == "12" and "windows-arm64" or "windows-x64"
  end
  local pipe = assert(io.popen("uname -s")); local name = pipe:read("*l") or ""; pipe:close()
  if name == "Darwin" then return "macos-arm64" end
  local architecture = assert(io.popen("uname -m")); local value = architecture:read("*l") or ""; architecture:close()
  return (value == "x86_64" or value == "amd64") and "linux-x64" or "linux-unsupported"
end
if target == "current" then target = host_target() end
if stage ~= "portable" and stage ~= "status" and stage ~= "propose" then assert(policy(target), "unknown target: " .. target) end
local function run(line) local ok, message = command.run(root, line, false); assert(ok, message) end
local function artifact(name)
  local release = ".heap/build/releases/" .. name
  return name:match("^windows") and release .. "/kero-installer.exe" or (name == "linux-x64" and release .. "/kero.AppImage" or release .. "/*.dmg")
end
local function static(name)
  run("KERO_PACKAGE_ARTIFACT=" .. command.quote(artifact(name)) .. " KERO_PACKAGE_STAGE_DIR=" .. command.quote(".heap/build/qt/" .. name .. "/stage")
    .. " " .. lua .. " distribution/tests/run.lua " .. command.quote(name))
end
local function machine()
  if package.config:sub(1, 1) ~= "\\" then local p = assert(io.popen("uname -m")); local v = p:read("*l") or "unknown"; p:close(); return v end
  return host_target() == "windows-arm64" and "arm64" or "x64"
end
local function evidence(name, runtime, adapter_name)
  local checksum = assert(io.open(root .. "/" .. artifact(name) .. ".sha256", "rb"), "missing checksum"):read("*l")
  local digest = assert(checksum:match("^([0-9a-fA-F]+)")):lower(); run("cmake -E make_directory .heap/distribution/evidence")
  local file = assert(io.open(root .. "/.heap/distribution/evidence/" .. name .. ".toml", "wb"))
  file:write('version = 1\ntarget = "', name, '"\nartifact = "', artifact(name), '"\nsha256 = "', digest, '"\n')
  file:write('builder_host = "', package.config:sub(1, 1) == "\\" and "windows" or "unix", '"\nbuilder_architecture = "', machine(), '"\n')
  file:write('build_mode = "', name == host_target() and "native" or "cross", '"\nstatic_validation = true\nruntime_mode = "', runtime, '"\nadapter = "', adapter_name, '"\n')
  file:write('toolchain = "', os.getenv("KERO_CXX_COMPILER") or (name == "windows-arm64" and "msvc" or "host"), '"\nqt_prefix = "', os.getenv("KERO_QT_PREFIX") or "", '"\nsigned = ', os.getenv("KERO_SIGN_RELEASE") == "1" and "true" or "false", "\n")
  file:close()
end
local function build(name)
  local item = assert(policy(name)); assert(item.enabled, name .. " is " .. item.lifecycle .. "; native macOS packaging is coming soon")
  if name == "windows-x64" and host_target() == "windows-arm64" and not os.getenv("KERO_CXX_COMPILER") then
    run("cmd.exe /d /s /c kero.cmd build windows-x64")
    return
  end
  run(lua .. " distribution/scripts/package.lua " .. command.quote(name)); static(name); evidence(name, "none", "none")
end
local function available_test(name, chosen)
  if chosen == "native" then return name == host_target(), "requires native " .. name .. " host" end
  if chosen == "emulated" or chosen == "windows-emulated" then return name == "windows-x64" and host_target() == "windows-arm64", "requires Windows ARM64 Prism emulation" end
  local configured = settings.adapter(root, chosen)
  if not configured then return false, "requires configured " .. chosen .. " adapter in distribution.local.kst" end
  return true
end
local function test(name, chosen)
  static(name); local ok, reason = available_test(name, chosen); assert(ok, reason)
  if chosen ~= "native" and chosen ~= "emulated" and chosen ~= "windows-emulated" then run(lua .. " distribution/scripts/adapter.lua " .. command.quote(chosen) .. " " .. command.quote(name) .. " test"); evidence(name, "vm", chosen); return end
  run("KERO_PACKAGE_ARTIFACT=" .. command.quote(artifact(name)) .. " KERO_PACKAGE_STAGE_DIR=" .. command.quote(".heap/build/qt/" .. name .. "/stage")
    .. " " .. lua .. " distribution/tests/run.lua " .. command.quote(name) .. " --runtime")
  evidence(name, chosen == "native" and "native" or "emulated", chosen)
end
local function portable()
  run(lua .. " distribution/tests/contract.lua"); run(lua .. " distribution/tests/local-pipeline.lua")
  if os.getenv("KERO_PORTABLE_WORKFLOW") == "1" then
    io.stdout:write("[ok] Portable workflow checks passed.\n")
  elseif command.run(root, "act --version", true) then
    run("act -j portable")
  else
    io.stdout:write("[skip] act is unavailable; portable Lua checks passed.\n")
  end
end
local function show_status()
  for _, item in ipairs(target_list()) do
    if not item.enabled then io.stdout:write(item.name, ": ", item.lifecycle, " (requires a Mac-native toolchain and contributor)\n")
    elseif item.name == host_target() then io.stdout:write(item.name, ": available native\n")
    elseif item.name == "windows-x64" and host_target() == "windows-arm64" then io.stdout:write(item.name, ": available cross-build; Prism test is opt-in\n")
    else io.stdout:write(item.name, ": unavailable (matching toolchain or adapter required)\n") end
  end
end
local function all()
  portable(); local untested, unavailable = {}, {}
  for _, item in ipairs(target_list()) do if item.enabled then
    if item.name == host_target() or (item.name == "windows-x64" and host_target() == "windows-arm64") then build(item.name); if item.name == host_target() then test(item.name, "native") else untested[#untested + 1] = item.name end
    else unavailable[#unavailable + 1] = item.name end
  end end
  run("cmake -E make_directory .heap/distribution"); local file = assert(io.open(root .. "/.heap/distribution/summary.toml", "wb")); file:write("complete = false\n")
  for _, name in ipairs(untested) do file:write('untested = "', name, '"\n') end; for _, name in ipairs(unavailable) do file:write('unavailable = "', name, '"\n') end; file:close()
  io.stdout:write("[partial] Native release evidence is incomplete.\n")
end
local function propose()
  local status = assert(command.capture(root, "git status --porcelain")); assert(status == "", "propose requires a clean committed source revision")
  local revision = assert(command.capture(root, "git rev-parse HEAD"))
  local records = {}
  for _, item in ipairs(target_list()) do if item.enabled then
    local e = assert(io.open(root .. "/.heap/distribution/evidence/" .. item.name .. ".toml", "rb"), "missing evidence for " .. item.name):read("*a")
    assert(e:match('runtime_mode = "native"'), "propose requires native runtime evidence for " .. item.name)
    records[#records + 1] = e
  end end
  run("cmake -E make_directory distribution/candidates")
  local file = assert(io.open(root .. "/distribution/candidates/" .. revision .. ".toml", "wb"))
  file:write('revision = "', revision, '"\n')
  for _, record in ipairs(records) do file:write("\n[[package]]\n", record) end
  file:close(); io.stdout:write("[ok] Candidate manifest written for native evidence.\n")
end
if stage == "portable" then portable() elseif stage == "status" then show_status() elseif stage == "build" then build(target) elseif stage == "verify" then static(target); evidence(target, "none", "none") elseif stage == "test" then test(target, adapter) elseif stage == "all" then all() else propose() end
