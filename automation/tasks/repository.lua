local manifest = require("lib.manifest")
local repository = {}
function repository.run(context)
  return manifest.require_paths(context.root, {
    ".github", "automation/run.lua", "code/Cargo.toml", "publication/publication.toml",
    "localization/locales.toml", "assets", "docs/README.md", "README.md",
  })
end
return repository
