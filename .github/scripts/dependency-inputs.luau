--!nonstrict
local runtime = require("../actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
for _, path in ipairs({ "src/Cargo.lock", ".github/dependabot.yml" }) do
  local file = io.open(path, "r")
  if not file then
    io.stderr:write("missing dependency input: " .. path .. "\n")
    os.exit(1)
  end
  file:close()
end

io.stdout:write("Dependency inputs are valid\n")
