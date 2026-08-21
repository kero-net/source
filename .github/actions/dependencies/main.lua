#!/usr/bin/env lua
local files = { "code/Cargo.lock", ".github/dependabot.yml" }
for _, path in ipairs(files) do
  local file = io.open(path, "r")
  if not file then io.stderr:write("missing dependency input: " .. path .. "\n"); os.exit(1) end
  file:close()
end
