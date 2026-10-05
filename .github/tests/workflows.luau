--!nonstrict
local runtime = require("../actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local function read(path)
  local file = assert(io.open(path, "r"), "missing " .. path)
  local value = file:read("*a"); file:close(); return value
end

local tasks = read(".vscode/tasks.json")
assert(tasks:match("Kero: Validate Locally"))
assert(tasks:match('"command": "powershell%.exe"'))
assert(tasks:match("distribution/actions/kero%-build%.ps1"))
assert(not tasks:match("kero all current"))

local action = read("distribution/actions/kero-build.sh")
assert(action:match("accepts no arguments"))
assert(action:match("build%-every%-distribution%.luau"))
assert(action:match("lune"))
local windows_action = read("distribution/actions/kero-build.ps1")
assert(windows_action:match("source%-checks%.luau local"))
assert(windows_action:match("wsl%.exe %-%-exec /bin/bash"))
assert(read("distribution/scripts/build/build-every-distribution.luau"):match("accepts no arguments"))

for _, directory in ipairs({ "windows-x86_64", "windows-aarch64", "linux-x86_64", "linux-aarch64" }) do
  local manifest = read("distribution/builds/" .. directory .. "/build.toml")
  assert(manifest:match("%[distribution%]"))
  assert(not manifest:match("%f[%w]lua%s*="))
end
local matrix = require("../../distribution/lib/hosted/create-github-build-matrix").Create(".")
assert(not matrix:match('"lua"'))
assert(not io.open("distribution/builds.json", "rb"))
assert(not io.open("distribution/scripts/package.luau", "rb"))
assert(not io.open("distribution/scripts/build-target.luau", "rb"))
assert(read("distribution/policy.toml"):match("%[verification%]"))
assert(read("distribution/tools/linuxdeploy.toml"):match("sha256"))
assert(read("distribution/tools/lune.toml"):match("0%.10%.5"))

local local_distribution = read(".github/workflows/local-distribution.yml")
assert(local_distribution:match("Local distribution portable checks"))
assert(local_distribution:match("lune"))
