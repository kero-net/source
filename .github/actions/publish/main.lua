#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/%.github/actions/publish/main%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local filesystem = require("lib.filesystem")
local releases = require("releases.validate")

local channel = arg[1]
local version = arg[2]
local source_commit = arg[3]
local payload = arg[4]
local token = os.getenv("PERSONAL_RELEASE_TOKEN") or os.getenv("GH_TOKEN")

local function fail(message)
  io.stderr:write(message .. "\n")
  os.exit(1)
end

if channel ~= "canary" and channel ~= "beta" and channel ~= "stable" then
  fail("unsupported publication channel: " .. tostring(channel))
end
if not releases.valid_id(version) then fail("invalid release ID: " .. tostring(version)) end
if not source_commit or not source_commit:match("^[0-9a-f]+$") or #source_commit ~= 40 then
  fail("source commit must be a full lowercase 40-character SHA")
end
if not payload or payload == "" then fail("publication payload is required") end
if not token or token == "" then fail("PERSONAL_RELEASE_TOKEN is required") end

local config, config_error = filesystem.read(root .. "/repo/config.toml")
if not config then fail(config_error) end
local target = config:match('target%s*=%s*"([^"]+)"')
if not target then fail("repo/config.toml is missing target") end

local release_record = root .. "/releases/records/" .. version .. ".md"
if not filesystem.is_file(release_record) then fail("missing authored release record: " .. version) end

local absolute_payload, payload_error = command.capture(root, "cd " .. command.quote(payload) .. " && pwd")
if not absolute_payload then fail(payload_error) end

local temporary = os.tmpname()
os.remove(temporary)
local made, make_error = command.run(root, "mkdir -p " .. command.quote(temporary), true)
if not made then fail(make_error) end

local function cleanup()
  command.run(root, "rm -rf " .. command.quote(temporary), true)
end

local function checked(ok, message)
  if not ok then cleanup(); fail(message) end
end

local remote = "https://x-access-token:" .. token .. "@github.com/" .. target .. ".git"
local worktree = temporary .. "/repository"
local clone_ok, clone_error = command.run_secret(
  root,
  "git clone --quiet --no-checkout " .. command.quote(remote) .. " " .. command.quote(worktree),
  "target repository clone"
)
checked(clone_ok, clone_error)

local exists = command.run(worktree, "git ls-remote --exit-code origin " .. command.quote("refs/heads/" .. channel) .. " >/dev/null 2>&1", true)
local expected = nil
if exists then
  local ok, message = command.run(worktree,
    "git fetch --quiet --no-tags origin "
      .. command.quote("refs/heads/" .. channel .. ":refs/remotes/origin/" .. channel), true)
  checked(ok, message)
  expected, message = command.capture(worktree, "git rev-parse " .. command.quote("origin/" .. channel))
  if not expected then cleanup(); fail(message) end
  ok, message = command.run(worktree,
    "git checkout --quiet --orphan " .. command.quote("generated-" .. channel) .. " " .. command.quote("origin/" .. channel), true)
  checked(ok, message)
else
  local ok, message = command.run(worktree,
    "git checkout --quiet --orphan " .. command.quote("generated-" .. channel), true)
  checked(ok, message)
end

checked(command.run(worktree, "git rm -rf . >/dev/null 2>&1 || true", true))
checked(command.run(worktree, "find . -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf -- {} +", true))
checked(command.run(worktree, "cp -R " .. command.quote(absolute_payload .. "/.") .. " .", true))
checked(command.run(worktree, "git add -A", true))
checked(command.run(worktree,
  "git commit -S -m " .. command.quote("Generate " .. channel .. " " .. version)
    .. " -m " .. command.quote("Source commit: " .. source_commit), true))
checked(command.run(worktree, "git verify-commit HEAD", true))

local push = "git push --quiet origin " .. command.quote("HEAD:refs/heads/" .. channel)
if expected then
  push = push .. " " .. command.quote("--force-with-lease=refs/heads/" .. channel .. ":" .. expected)
else
  push = push .. " --force"
end
checked(command.run(worktree, push, true))

local generated_commit, commit_error = command.capture(worktree, "git rev-parse HEAD")
if not generated_commit then cleanup(); fail(commit_error) end

local tag = "v" .. version
if command.run(worktree, "git ls-remote --exit-code --tags origin " .. command.quote("refs/tags/" .. tag) .. " >/dev/null 2>&1", true) then
  cleanup(); fail(tag .. " already exists; release IDs are immutable")
end
if command.run(root, "gh release view " .. command.quote(tag) .. " --repo " .. command.quote(target) .. " >/dev/null 2>&1", true) then
  cleanup(); fail(tag .. " GitHub Release already exists; release IDs are immutable")
end

checked(command.run(worktree, "git tag -s -m " .. command.quote(tag) .. " " .. command.quote(tag) .. " HEAD", true))
checked(command.run(worktree, "git verify-tag " .. command.quote(tag), true))
checked(command.run(worktree, "git push --quiet origin " .. command.quote("refs/tags/" .. tag .. ":refs/tags/" .. tag), true))

local archive = temporary .. "/" .. tag .. ".tar.gz"
checked(command.run(root,
  "tar -czf " .. command.quote(archive) .. " -C " .. command.quote(absolute_payload) .. " .", true))

local release = "gh release create " .. command.quote(tag)
  .. " " .. command.quote(archive)
  .. " --repo " .. command.quote(target)
  .. " --verify-tag --notes-file " .. command.quote(release_record)
  .. " --title " .. command.quote(version)
if channel == "canary" or channel == "beta" then
  release = release .. " --prerelease --latest=false"
else
  release = release .. " --latest"
end
checked(command.run(root, release, true))

local output = os.getenv("GITHUB_OUTPUT")
if output and output ~= "" then
  local file, output_error = io.open(output, "a")
  if not file then cleanup(); fail(output_error) end
  file:write("generated_commit=", generated_commit, "\n")
  file:close()
end

cleanup()
io.stdout:write("Published ", channel, " ", version, " to ", target, " at ", generated_commit, "\n")
