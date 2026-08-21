local filesystem = require("lib.filesystem")
local json = {}
function json.validate_file(path)
  local value, message = filesystem.read(path); if not value then return false, message end
  if value:sub(1, 1) ~= "{" or value:match("}%s*$") == nil then return false, "invalid JSON object: " .. path end
  return true
end
return json
