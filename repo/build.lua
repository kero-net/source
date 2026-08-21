#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/repo/build%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local filesystem = require("lib.filesystem")
local i18n = require("i18n.render")
local pages = require("pages.render")

local function read_config()
  local value, message = filesystem.read(root .. "/repo/config.toml")
  if not value then return nil, message end
  local target = value:match('target%s*=%s*"([^"]+)"')
  local channels = {}
  local body = value:match("channels%s*=%s*%[(.-)%]")
  if body then
    for channel in body:gmatch('"([^"]+)"') do channels[channel] = true end
  end
  if not target or next(channels) == nil then return nil, "invalid repo/config.toml" end
  return { target = target, channels = channels }
end

local function locale_navigation(locales)
  local cells = {}
  for _, locale in ipairs(locales) do
    local file = locale == locales[1] and "README.md" or ("README." .. locale.key .. ".md")
    cells[#cells + 1] = string.format('<td><a href="%s">%s</a></td>', file, locale.language)
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

local destination = arg[2] or ("../.heap/source/repo/" .. channel)
local version = arg[3] or "unreleased"
local source_commit = arg[4]
if not source_commit or source_commit == "" then
  source_commit = command.capture(root, "git rev-parse HEAD") or "unknown"
end

local ok, message = command.run(root, "rm -rf " .. command.quote(destination) .. " && mkdir -p " .. command.quote(destination), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

for _, directory in ipairs({ "code", "assets" }) do
  ok, message = command.run(root, "cp -R " .. command.quote(directory) .. " " .. command.quote(destination .. "/" .. directory), true)
  if not ok then io.stderr:write(message .. "\n"); os.exit(1) end
end

ok, message = command.run(root, "mkdir -p " .. command.quote(destination .. "/releases")
  .. " && cp -R releases/records/. " .. command.quote(destination .. "/releases/"), true)
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
local navigation = locale_navigation(locales)

for index, locale in ipairs(locales) do
  local rendered, render_error = i18n.text(root, template, locale.key)
  if not rendered then io.stderr:write(render_error .. "\n"); os.exit(1) end
  rendered = rendered:gsub("{{%s*locales:repository%s*}}", navigation)
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
