local filesystem = require("lib.filesystem")
local manifest = {}
function manifest.require_paths(root, paths)
  for _, relative in ipairs(paths) do
    if not filesystem.exists(root .. "/" .. relative) then return false, "missing required path: " .. relative end
  end
  return true
end
return manifest
