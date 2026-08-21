local script = (arg and arg[0] or ""):gsub("\\", "/")
local root = script:match("^(.*)/i18n/validate%.lua$") or "."
package.path = root .. "/?.lua;" .. package.path

local renderer = require("i18n.render")
local i18n = {}

function i18n.validate(base)
  local locales, message = renderer.locales(base)
  if not locales then return false, message end

  local pipe = io.popen("find " .. string.format("%q", base .. "/i18n") .. " -maxdepth 1 -type f -name '*.toml' -printf '%f\\n' | sort", "r")
  if not pipe then return false, "unable to enumerate localization catalogs" end
  for filename in pipe:lines() do
    if filename ~= "locales.toml" then
      local name = filename:gsub("%.toml$", "")
      local _, catalog_error = renderer.catalog(base, name)
      if catalog_error then pipe:close(); return false, catalog_error end
    end
  end
  pipe:close()
  return true
end

if script:match("i18n/validate%.lua$") then
  local ok, message = i18n.validate(root)
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end

return i18n
