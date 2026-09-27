local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/releases/build%.lua$") or "."
package.path = root .. "/?.lua;" .. package.path

local releases = require("releases.validate")
local destination = arg[1]
if not destination or destination == "" then
  io.stderr:write("usage: lua5.4 releases/build.lua DESTINATION\n")
  os.exit(2)
end

local stream = assert(io.popen("find " .. string.format("%q", root .. "/releases/records")
  .. " -maxdepth 1 -type f -name '*.md' -printf '%f\\n' | sort -r"))
local records = {}
for name in stream:lines() do
  local id = name:match("^(.*)%.md$")
  if not releases.valid_id(id) then
    stream:close()
    io.stderr:write("invalid release record filename: ", name, "\n")
    os.exit(1)
  end
  records[#records + 1] = { id = id, path = root .. "/releases/records/" .. name }
end
stream:close()

local output = { "# Changelog\n" }
for _, record in ipairs(records) do
  local file = assert(io.open(record.path, "rb"))
  output[#output + 1] = "\n" .. file:read("*a"):gsub("%s+$", "") .. "\n"
  file:close()
end
local file = assert(io.open(destination, "wb"))
file:write(table.concat(output))
file:close()
