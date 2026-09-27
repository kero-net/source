package.path = "./?.lua;" .. package.path
local settings = require("distribution.lib.settings")
local policy = settings.read(".")

for _, target in ipairs({ "windows-arm64", "windows-x64", "linux-x64", "macos-arm64" }) do
  local section = policy.targets[target]
  assert(section, "missing distribution target: " .. target)
  assert(section.lifecycle ~= "", target .. " must declare lifecycle")
  if target == "macos-arm64" then
    assert(not section.enabled, "macos-arm64 must remain coming soon")
    assert(section.lifecycle == "coming-soon", "macos-arm64 must be coming soon")
  else
    assert(section.enabled, target .. " must be enabled")
  end
  assert(section.artifact ~= "", target .. " must declare an artifact")
  assert(section.native_host ~= "", target .. " must declare a native host")
  assert(section.required_environment ~= nil, target .. " must declare required environment")
end

local runner = assert(io.open("distribution/scripts/local-run.lua", "rb")):read("*a")
for _, stage in ipairs({ "portable", "build", "verify", "test", "all", "propose", "status" }) do
  assert(runner:find(stage .. " = true", 1, true), "missing stage: " .. stage)
end
assert(runner:find('runtime_mode = "', 1, true))
assert(runner:find('build_mode = "', 1, true))
assert(runner:find('chosen == "emulated"', 1, true))
assert(runner:find("distribution.local.kst", 1, true))
assert(not runner:find("distribution/distribution.toml", 1, true))
assert(runner:find("propose requires native runtime evidence", 1, true))
assert(runner:find("Candidate manifest written", 1, true))
assert(not runner:find("KERO_CI_REPOSITORY", 1, true))
assert(assert(io.open("distribution/scripts/adapter.lua", "rb")):read("*a"):find("QEMU adapter", 1, true))
assert(assert(io.open("distribution.local.kst.example", "rb")):read("*a"):find("adapter qemu", 1, true))
assert(assert(io.open("kero.cmd", "rb")):read("*a"):find("distribution\\actions\\kero.cmd", 1, true))
local wrapper = assert(io.open("distribution/actions/kero.cmd", "rb")):read("*a")
assert(wrapper:find("x86_64-w64-mingw32-clang++", 1, true))
local package = assert(io.open("distribution/scripts/package.lua", "rb")):read("*a")
assert(package:find('KERO_SIGN_RELEASE") == "1"', 1, true))
