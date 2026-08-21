local manifest = require("lib.manifest")
local release = {}
function release.run(context)
  local version = context.args[2]; if not version then return false, "release requires a version" end
  return manifest.require_paths(context.root, { "publication/releases/" .. version .. ".release.md" })
end
return release
