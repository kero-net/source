#!/usr/bin/env lua
local required = {
  "code/Cargo.toml",
  "code/crates/kero-core/Cargo.toml",
  "code/crates/kero-cli/Cargo.toml",
  "assets/images",
  "releases/records",
  "i18n/locales.toml",
  "i18n/documentation.toml",
  "i18n/getting-started.toml",
  "i18n/repository.toml",
  "pages/build.lua",
  "pages/render.lua",
  "repo/build.lua",
  "repo/config.toml",
  ".github/actions",
  ".cargo/config.toml",
  "README.md",
  "LICENSE",
}

local function exists(path)
  local file = io.open(path, "r")
  if file then file:close(); return true end
  return os.rename(path, path) ~= nil
end

for _, path in ipairs(required) do
  if not exists(path) then
    io.stderr:write("missing required path: " .. path .. "\n")
    os.exit(1)
  end
end

for _, path in ipairs({ "automation", "publication", "docs", "localization", "content", "builders" }) do
  if exists(path) then
    io.stderr:write("legacy source directory still exists: " .. path .. "\n")
    os.exit(1)
  end
end

io.stdout:write("Repository shape is valid\n")
