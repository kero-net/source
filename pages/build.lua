#!/usr/bin/env lua
local script = arg[0]:gsub("\\", "/")
local root = script:match("^(.*)/pages/build%.lua$") or "."
package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path

local command = require("lib.command")
local filesystem = require("lib.filesystem")
local renderer = require("i18n.render")
local pages = require("pages.render")

local destination = arg[1] or "../.heap/source/pages/site"
local staging = arg[2] or "../.heap/source/pages/markdown"

local ok, message = pages.render(root, staging)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

local locales, locale_error = renderer.locales(root)
if not locales then io.stderr:write(locale_error .. "\n"); os.exit(1) end
local default_locale = locales[1].key

ok, message = command.run(root, "command -v pandoc >/dev/null 2>&1")
if not ok then io.stderr:write("pages build requires pandoc\n"); os.exit(1) end
ok, message = command.run(root, "rm -rf " .. command.quote(destination) .. " && mkdir -p " .. command.quote(destination), true)
if not ok then io.stderr:write(message .. "\n"); os.exit(1) end

for _, locale in ipairs(locales) do
  local locale_root = staging .. "/" .. locale.key
  local listing, list_error = command.capture(root, "find " .. command.quote(locale_root) .. " -type f -name '*.md' -printf '%P\\n' | sort")
  if not listing then io.stderr:write(list_error .. "\n"); os.exit(1) end

  for relative in (listing .. "\n"):gmatch("(.-)\n") do
    if relative ~= "" then
      local input = locale_root .. "/" .. relative
      local output_relative = relative:gsub("%.md$", ".html")
      local output_root = destination
      if locale.key ~= default_locale then output_root = output_root .. "/" .. locale.key end
      local output = output_root .. "/" .. output_relative
      local parent = output:match("^(.*)/[^/]+$")
      local made, mkdir_error = command.run(root, "mkdir -p " .. command.quote(parent), true)
      if not made then io.stderr:write(mkdir_error .. "\n"); os.exit(1) end

      local markdown, read_error = filesystem.read(input)
      if not markdown then io.stderr:write(read_error .. "\n"); os.exit(1) end
      markdown = markdown:gsub("%.md%)", ".html)")
      local temporary = staging .. "/.pandoc-" .. locale.key .. "-" .. relative:gsub("[^%w%-]", "_")
      local wrote, write_error = filesystem.write(temporary, markdown)
      if not wrote then io.stderr:write(write_error .. "\n"); os.exit(1) end

      local pandoc = "pandoc --standalone --from=gfm -V lang=" .. command.quote(locale.key)
        .. " -V pagetitle=KERO --output " .. command.quote(output) .. " " .. command.quote(temporary)
      local built, build_error = command.run(root, pandoc)
      os.remove(temporary)
      if not built then io.stderr:write(build_error .. "\n"); os.exit(1) end
    end
  end
end

if command.run(root, "test -d assets", true) then
  command.run(root, "mkdir -p " .. command.quote(destination .. "/assets")
    .. " && cp -R assets/. " .. command.quote(destination .. "/assets/"), true)
end

local language_index = { "<!doctype html>", "<meta charset=\"utf-8\">", "<title>KERO documentation languages</title>", "<h1>KERO documentation</h1>", "<ul>" }
for _, locale in ipairs(locales) do
  local prefix = locale.key == default_locale and "" or (locale.key .. "/")
  language_index[#language_index + 1] = string.format('<li><a href="%sindex.html">%s (%s)</a></li>', prefix, locale.language, locale.key)
end
language_index[#language_index + 1] = "</ul>"
filesystem.write(destination .. "/languages.html", table.concat(language_index, "\n") .. "\n")
