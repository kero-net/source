--!nonstrict
local runtime = require("../.github/actions/lib/legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local script = ("repo/build.luau"):gsub("\\", "/")
local root = script:match("^(.*)/repo/build%.luau$") or "."

local command = require("../.github/actions/lib/command")
local filesystem = require("../.github/actions/lib/filesystem")
local kst = require("../.github/actions/lib/kst")
local i18n = require("../i18n/render")
local pages = require("../pages/render")

local function read_config()
  local ok, document = pcall(kst.parse_file, root .. "/repo/config.kst")
  if not ok then return nil, document end
  local repository = kst.child(document, "repository")
  if not repository then return nil, "repo/config.kst is missing repository" end
  local target_node = kst.child(repository, "target")
  local target = target_node and target_node.value
  local channels = {}
  for _, node in ipairs(kst.children(repository, "channel")) do channels[node.value] = true end
  if not target or next(channels) == nil then return nil, "invalid repo/config.kst" end
  return { target = target, channels = channels }
end

local function locale_navigation(locales, active_locale)
  local cells = {}
  for index, locale in ipairs(locales) do
    if locale.key ~= active_locale then
      local file = index == 1 and "README.md" or ("README." .. locale.key .. ".md")
      cells[#cells + 1] = string.format('<td><a href="%s">%s</a></td>', file, locale.language)
    end
  end
  return "<table><tr>" .. table.concat(cells) .. "</tr></table>"
end

local config, config_error = read_config()
if not config then io.stderr:write(config_error .. "\n"); os.exit(1) end

local channel = arg[1] or "canary"
if not config.channels[channel] then
  io.stderr:write("unsupported publication channel: " .. tostring(channel) .. "\n")
  os.exit(2)
end

local destination = arg[2] or (".heap/repo/" .. channel)
local version = arg[3] or "unreleased"
local source_commit = arg[4]
if not source_commit or source_commit == "" then
  source_commit = command.capture(root, "git rev-parse HEAD") or "unknown"
end

local ok, message = command.run(root, "rm -rf " .. command.quote(destination) .. " && mkdir -p " .. command.quote(destination), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

-- `src/target` is compiler output, not repository source.  Copy source through
-- tar so the publication tree cannot accidentally inherit a locally-created
-- target directory (which can otherwise be gigabytes per publication channel).
ok, message = command.run(root,
  "mkdir -p " .. command.quote(destination .. "/src")
    .. " && (cd src && tar --exclude='./target' -cf - .)"
    .. " | (cd " .. command.quote(destination .. "/src") .. " && tar -xf -)", true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

ok, message = command.run(root, "cp -R assets " .. command.quote(destination .. "/assets"), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

ok, message = command.run(root, "mkdir -p " .. command.quote(destination .. "/releases")
  .. " && cp -R releases/records/. " .. command.quote(destination .. "/releases/"), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

ok, message = command.run(root, "lune run releases/build.luau " .. command.quote(destination .. "/CHANGELOG.md"), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

for _, file in ipairs({ "LICENSE", "CITATION.cff" }) do
  ok, message = command.run(root, "cp " .. command.quote(file) .. " " .. command.quote(destination .. "/" .. file), true)
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end

for _, file in ipairs({ "CONTRIBUTING.md", "SECURITY.md", "CODE_OF_CONDUCT.md" }) do
  ok, message = command.run(root, "cp " .. command.quote("repo/templates/" .. file) .. " " .. command.quote(destination .. "/" .. file), true)
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end

local locales, locale_error = i18n.locales(root)
if not locales then io.stderr:write(locale_error .. "\n"); os.exit(1) end
local template, template_error = filesystem.read(root .. "/repo/templates/README.md")
if not template then io.stderr:write(template_error .. "\n"); os.exit(1) end
for index, locale in ipairs(locales) do
  local rendered, render_error = i18n.text(root, template, locale.key)
  if not rendered then io.stderr:write(render_error .. "\n"); os.exit(1) end
  rendered = rendered:gsub("{{%s*locales:repository%s*}}", locale_navigation(locales, locale.key))
  local file = index == 1 and "README.md" or ("README." .. locale.key .. ".md")
  local written, write_error = filesystem.write(destination .. "/" .. file, rendered)
  if not written then io.stderr:write(write_error .. "\n"); os.exit(1) end
end

ok, message = pages.render(root, destination .. "/docs")
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

local publication = string.format(
  'channel = "%s"\nversion = "%s"\nsource = "%s"\n',
  channel, version, source_commit
)
local written, write_error = filesystem.write(destination .. "/publication.toml", publication)
if not written then io.stderr:write(write_error .. "\n"); os.exit(1) end

local unresolved, grep_error = command.capture(root,
  "grep -RIl --exclude='*.toml' '{{[[:space:]]*l10n:' " .. command.quote(destination) .. " || true")
if unresolved and unresolved ~= "" then
  io.stderr:write("unresolved localization keys in generated repository:\n" .. unresolved .. "\n")
  os.exit(1)
end

io.stdout:write("Built ", channel, " repository at ", destination, "\n")
