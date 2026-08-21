local filesystem = require("lib.filesystem")
local toml = {}
function toml.validate_file(path)
  local value, message = filesystem.read(path); if not value then return false, message end
  if value:find("%z") then return false, "invalid TOML: " .. path end
  return true
end
return toml
