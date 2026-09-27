#!/usr/bin/env lua
-- Runs source-owned checks and generated projections without publishing.
-- Publication stays GitHub-only because it requires protected organization credentials.
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/pipeline%.lua$")
if not root or root == "" then root = "." end
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local lua = os.getenv("KERO_LUA") or "lua"
local is_windows = package.config:sub(1, 1) == "\\"
local cargo = is_windows and "cargo +stable-aarch64-pc-windows-gnullvm " or "cargo "
-- These checks are a gate: no generated projection is allowed to start until
-- the source is known to be valid.  The projections below have disjoint output
-- directories, so they intentionally run together after this gate.
local checks = {
  cargo .. "fmt --manifest-path src/Cargo.toml --all -- --check",
  cargo .. "clippy --manifest-path src/Cargo.toml --all-targets --all-features --locked -- -D warnings",
  cargo .. "test --manifest-path src/Cargo.toml --all-targets --all-features --locked",
  lua .. " i18n/validate.lua",
  lua .. " releases/test.lua",
  lua .. " .github/tests/workflows.lua",
}

local projections = {
  lua .. " repo/build.lua canary .heap/build/repo/canary",
  lua .. " repo/build.lua beta .heap/build/repo/beta",
  lua .. " repo/build.lua stable .heap/build/repo/stable",
  lua .. " pages/build.lua .heap/build/pages",
}

local clean_heap = "cmake -E rm -rf .heap && cmake -E make_directory .heap"

io.stdout:write("\n==> ", clean_heap, "\n")
local clean_ok, clean_message = command.run(root, clean_heap, true)
if not clean_ok then io.stderr:write(clean_message, "\n"); os.exit(1) end

for _, step in ipairs(checks) do
  io.stdout:write("\n==> ", step, "\n")
  local ok, message = command.run(root, step, true)
  if not ok then io.stderr:write(message, "\n"); os.exit(1) end
end

for _, step in ipairs(projections) do
  io.stdout:write("\n==> ", step, "\n")
  local ok, message = command.run(root, step, true)
  if not ok then io.stderr:write(message, "\n"); os.exit(1) end
end

local release = lua .. " distribution/scripts/package.lua"
for _, value in ipairs(arg) do release = release .. " " .. command.quote(value) end
io.stdout:write("\n==> ", release, "\n")
local release_ok, release_message = command.run(root, release, true)
if not release_ok then io.stderr:write(release_message, "\n"); os.exit(1) end

io.stdout:write("\nLocal publication pipeline completed. Push source to let GitHub perform protected publication.\n")
