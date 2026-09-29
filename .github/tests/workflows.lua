local function read(path)
  local file = assert(io.open(path, "r"), "missing " .. path)
  local value = file:read("*a"); file:close(); return value
end

local tasks = read(".vscode/tasks.json")
assert(tasks:match("Kero: Validate Locally"))
assert(tasks:match('"command": "wsl%.exe"'))
assert(tasks:match('"%-%-exec"'))
assert(tasks:match('"/bin/bash"'))
assert(tasks:match("distribution/actions/kero%-build%.sh"))
assert(not tasks:lower():match("powershell"))
assert(not tasks:match("kero all current"))

local action = read("distribution/actions/kero-build.sh")
assert(action:match("accepts no arguments"))
assert(action:match("build%-every%-distribution%.luau"))
assert(action:match("lune"))
assert(not io.open("distribution/actions/kero-build.ps1", "rb"))
assert(read("distribution/scripts/build/build-every-distribution.luau"):match("accepts no arguments"))

for _, directory in ipairs({ "windows-x86_64", "windows-aarch64", "linux-x86_64", "linux-aarch64" }) do
  assert(read("distribution/builds/" .. directory .. "/build.toml"):match("%[distribution%]"))
end
assert(not io.open("distribution/builds.json", "rb"))
assert(not io.open("distribution/scripts/package.lua", "rb"))
assert(not io.open("distribution/scripts/build-target.lua", "rb"))
assert(read("distribution/policy.toml"):match("%[verification%]"))
assert(read("distribution/tools/linuxdeploy.toml"):match("sha256"))
assert(read("distribution/tools/lune.toml"):match("0%.10%.5"))

local local_distribution = read(".github/workflows/local-distribution.yml")
assert(local_distribution:match("Local distribution portable checks"))
assert(local_distribution:match("lune"))
