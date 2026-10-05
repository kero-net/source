--!nonstrict
local runtime = require("../actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local required = {
  "src/Cargo.toml",
  "src/crates/kero-core/Cargo.toml",
  "src/crates/kero-cli/Cargo.toml",
  "assets/images",
  "releases/records",
  "i18n/locales.toml",
  "i18n/documentation.toml",
  "i18n/getting-started.toml",
  "i18n/repository.toml",
  "pages/build.luau",
  "pages/render.luau",
  "repo/build.luau",
  "repo/config.kst",
  ".github/actions",
  ".kero/README.md",
  "README.md",
  "LICENSE",
}

local function exists(path)
  local file = io.open(path, "r")
  if file then file:close(); return true end
  return os.rename(path, path) == true
end

for _, path in ipairs(required) do
  print("Checking required path: " .. path)
  if not exists(path) then
    io.stderr:write("missing required path: " .. path .. "\n")
    os.exit(1)
  end
end

for _, path in ipairs({ "code", "automation", "publication", "docs", "localization", "content", "builders" }) do
  print("Checking obsolete path is absent: " .. path)
  if exists(path) then
    io.stderr:write("legacy source directory still exists: " .. path .. "\n")
    os.exit(1)
  end
end

io.stdout:write("Repository shape is valid\n")
