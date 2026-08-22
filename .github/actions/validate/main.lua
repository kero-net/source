#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/actions/validate/main%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
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

for _, path in ipairs(required) do
  local file = io.open(root .. "/" .. path, "r")
  if file then
    file:close()
  elseif not os.rename(root .. "/" .. path, root .. "/" .. path) then
    io.stderr:write("missing required path: " .. path .. "\n")
    os.exit(1)
  end
end

for _, removed in ipairs({ "automation", "publication", "docs", "localization", "content", "builders" }) do
  if os.rename(root .. "/" .. removed, root .. "/" .. removed) then
    io.stderr:write("legacy source directory still exists: " .. removed .. "\n")
    os.exit(1)
  end
end

local commands = {
  "git diff --check",
  "find pages repo releases i18n .github/actions -name '*.lua' -print0 | xargs -0 -n1 luac5.4 -p",
  "lua5.4 i18n/validate.lua",
  "lua5.4 releases/test.lua",
  "lua5.4 .github/actions/publish/test.lua",
  "lua5.4 pages/build.lua ../.heap/source/pages/site",
  "lua5.4 repo/build.lua canary ../.heap/source/repo/canary",
  "groff -z -mandoc code/man/kero.1",
  "cargo fmt --manifest-path code/Cargo.toml --all --check",
  "cargo clippy --manifest-path code/Cargo.toml --all-targets --all-features -- -D warnings",
  "cargo test --manifest-path code/Cargo.toml --all-targets --locked",
}

for _, program in ipairs(commands) do
  local ok, message = command.run(root, program)
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end
