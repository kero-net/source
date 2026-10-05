--!nonstrict
local runtime = require("./actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
-- Runs source-owned checks and generated projections without publishing.
-- Publication stays GitHub-only because it requires protected organization credentials.
local script = (".github/pipeline.luau"):gsub("\\", "/")
local root = script:match("^(.*)/%.github/pipeline%.luau$")
if not root or root == "" then root = "." end

local command = require("./actions/lib/command")
local lune = "lune run"
local is_windows = require("@lune/process").os == "windows"
local cargo = is_windows and "cargo +stable-aarch64-pc-windows-gnullvm " or "cargo "
-- These checks are a gate: no generated projection is allowed to start until
-- the source is known to be valid.  The projections below have disjoint output
-- directories, so they intentionally run together after this gate.
local checks = {
  cargo .. "fmt --manifest-path src/Cargo.toml --all -- --check",
  cargo .. "clippy --manifest-path src/Cargo.toml --all-targets --all-features --locked -- -D warnings",
  cargo .. "test --manifest-path src/Cargo.toml --all-targets --all-features --locked",
  lune .. " i18n/validate.luau",
  lune .. " releases/test.luau",
  lune .. " .github/tests/workflows.luau",
}

local projections = {
  lune .. " repo/build.luau canary .heap/build/repo/canary",
  lune .. " repo/build.luau beta .heap/build/repo/beta",
  lune .. " repo/build.luau stable .heap/build/repo/stable",
  lune .. " pages/build.luau .heap/build/pages",
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

io.stdout:write("\nLocal publication pipeline completed. Push source to let GitHub perform protected publication.\n")
