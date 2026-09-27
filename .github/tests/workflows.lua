local function read(path)
  local file = assert(io.open(path, "r"))
  local value = file:read("*a")
  file:close()
  return value
end

local ci = read(".github/workflows/ci.yml")
assert(ci:match("name: CI"))
assert(ci:match("name: CI Gate"))
assert(ci:match("name: Publication Decision"))
assert(ci:match("uses: %./%.github/workflows/release%.yml"))
assert(ci:match("rustup target add wasm32%-wasip1"))

local publication = read(".github/workflows/release.yml")
assert(publication:match("name: Publication"))
assert(publication:match("workflow_call:"))
assert(not publication:match("workflow_dispatch:"))
assert(publication:match("channel: %[canary, beta, stable%]"))
assert(publication:match("owner: kero%-net"))
assert(publication:match("repositories: kero"))
assert(publication:match("environment: release"))

local record = read("releases/publication.json")
assert(record:match('"enabled"%s*:%s*true') or record:match('"enabled"%s*:%s*false'))
assert(record:match('"record"%s*:%s*"2026%.08%.1%-regular"'))

local pipeline = read(".github/pipeline.lua")
assert(pipeline:match("distribution/scripts/package%.lua"))
assert(not pipeline:lower():match("powershell"))

local presets = read("host/qt/CMakePresets.json")
for _, target in ipairs({ "windows-arm64", "windows-x64", "linux-x64", "macos-arm64" }) do
  assert(presets:find('"' .. target .. '"', 1, true))
end
local qt_cmake = read("host/qt/CMakeLists.txt")
assert(qt_cmake:match("stage/usr/bin/kero%-install"))
assert(qt_cmake:match("resources/kero%.desktop"))

local files = assert(io.popen("git ls-files '*.ps1'"))
assert(files:read("*a") == "", "PowerShell scripts must not be tracked")
files:close()

local local_distribution = read(".github/workflows/local-distribution.yml")
assert(local_distribution:match("Local distribution portable checks"))
assert(local_distribution:match("KERO_PORTABLE_WORKFLOW=1"))
assert(local_distribution:match("distribution/scripts/local%-run%.lua portable"))
local distribution = read("distribution/distribution.kst")
for _, target in ipairs({ "windows-arm64", "windows-x64", "linux-x64", "macos-arm64" }) do
  assert(distribution:find("target " .. target, 1, true))
end
assert(distribution:match("nativeHost windows%-arm64"))
assert(distribution:match("requiredEnvironment KERO_QT_PREFIX,KERO_RUNTIME_DIR,KERO_WINDRES,KERO_CXX_COMPILER"))
assert(read("distribution/tests/run.lua"):match("distribution/tests/contract%.lua"))
assert(read("distribution/scripts/local-run.lua"):match("%.heap/distribution"))
