#!/usr/bin/env lua
-- Shared credential-free source checks for CI and local validation.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/scripts/source%-checks%.lua$") or "."
package.path = root .. "/.github/actions/?.lua;" .. package.path
local command = require("lib.command")
local lua = os.getenv("KERO_LUA") or "lua"
local luac = os.getenv("KERO_LUAC") or "luac"
local cargo = os.getenv("KERO_CARGO") or "cargo"
local group = arg[1] or "local"

local checks = {
  contracts = { "" .. lua .. " .github/scripts/repository-contracts.lua", "git diff --check" },
  rust = {
    cargo .. " fmt --manifest-path src/Cargo.toml --all --check",
    cargo .. " clippy --manifest-path src/Cargo.toml --all-targets --all-features --locked -- -D warnings",
    cargo .. " test --manifest-path src/Cargo.toml --all-targets --locked",
  },
  automation = {
    "find .github -name '*.lua' -print0 | xargs -0 -n1 " .. luac .. " -p",
    lua .. " .github/tests/workflows.lua",
    lua .. " .github/scripts/actionlint.lua",
  },
  localization = { lua .. " i18n/validate.lua" },
  release = {
    lua .. " releases/tests/id.lua", lua .. " releases/tests/records.lua", lua .. " releases/tests/sequence.lua",
    lua .. " .github/actions/publish/tests/policy.lua", lua .. " .github/actions/publish/tests/preflight.lua",
  },
  documentation = { "groff -z -mandoc src/man/kero.1" },
  pages = { lua .. " pages/build.lua .heap/pages" },
  repository = {
    lua .. " repo/build.lua stable .heap/repo/stable unreleased",
    lua .. " repo/build.lua beta .heap/repo/beta unreleased",
    lua .. " repo/build.lua canary .heap/repo/canary unreleased",
  },
}

local function run(name, optional)
  for _, line in ipairs(assert(checks[name], "unknown source-check group: " .. name)) do
    local ok, message = command.run(root, line, false)
    if not ok then
      if optional then
        io.stdout:write("[skip] ", name, " needs local prerequisite: ", message, "\n")
        return
      end
      io.stderr:write(message, "\n")
      os.exit(1)
    end
  end
end

if group == "local" then
  for _, name in ipairs({ "contracts", "rust", "automation", "localization", "release" }) do run(name) end
  run("documentation", true)
  run("pages", true)
  run("repository", true)
else
  run(group)
end
