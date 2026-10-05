--!nonstrict
local runtime = require("../actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
-- Shared credential-free source checks for CI and local validation.
local script = (".github/scripts/source-checks.luau"):gsub("\\", "/")
local root = script:match("^(.*)/%.github/scripts/source%-checks%.luau$") or "."
local command = require("../actions/lib/command")
local lune = "lune run"
local cargo = os.getenv("KERO_CARGO") or "cargo"
local group = arg[1] or "local"

local checks = {
  contracts = { "" .. lune .. " .github/scripts/repository-contracts.luau", "git diff --check" },
  rust = {
    cargo .. " fmt --manifest-path src/Cargo.toml --all --check",
    cargo .. " clippy --manifest-path src/Cargo.toml --all-targets --all-features --locked -- -D warnings",
    cargo .. " test --manifest-path src/Cargo.toml --all-targets --locked",
  },
  automation = {
    lune .. " .github/tests/workflows.luau",
    lune .. " .github/scripts/actionlint.luau",
  },
  localization = { lune .. " i18n/validate.luau" },
  release = {
    lune .. " releases/tests/id.luau", lune .. " releases/tests/records.luau", lune .. " releases/tests/sequence.luau",
    lune .. " .github/actions/publish/tests/policy.luau", lune .. " .github/actions/publish/tests/preflight.luau",
  },
  documentation = { "groff -z -mandoc src/man/kero.1" },
  pages = { lune .. " pages/build.luau .heap/pages" },
  repository = {
    lune .. " repo/build.luau stable .heap/repo/stable unreleased",
    lune .. " repo/build.luau beta .heap/repo/beta unreleased",
    lune .. " repo/build.luau canary .heap/repo/canary unreleased",
  },
}

local function run(name, optional)
  print(":: " .. name .. " checks")
  for _, line in ipairs(assert(checks[name], "unknown source-check group: " .. name)) do
    local ok, message = command.run(root, line, false)
    if not ok then
      if optional then
        io.stdout:write("- ", name, " needs local prerequisite: ", message, "\n")
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
