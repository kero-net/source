local manifest = require("lib.manifest")
local localization = {}
function localization.run(context)
  return manifest.require_paths(context.root, { "localization/locales.toml", "publication/repository/README.template.md", "publication/repository/metadata.toml" })
end
return localization
