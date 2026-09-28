local invocation = "lua .github/actions/publish/main.lua canary 2026.08.1-regular "
  .. "0123456789012345678901234567890123456789 . 2>&1"
local command = package.config:sub(1, 1) == "\\"
  and "set \"KERO_RELEASE_TOKEN=\" && set \"GH_TOKEN=\" && " .. invocation
  or "env -u KERO_RELEASE_TOKEN -u GH_TOKEN " .. invocation
local process = assert(io.popen(command))
local output = process:read("*a")
local ok = process:close()
assert(not ok)
assert(output:match("KERO_RELEASE_TOKEN is required"))
