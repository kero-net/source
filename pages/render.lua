local pages = {}

local function configure(root)
  package.path = root .. "/?.lua;" .. root .. "/.github/actions/?.lua;" .. package.path
end

local function template_destination(relative)
  if relative:match("^documentation/") then
    return relative:gsub("^documentation/", "", 1)
  end
  return relative
end

function pages.render(root, destination)
  configure(root)
  local command = require("lib.command")
  local filesystem = require("lib.filesystem")
  local i18n = require("i18n.render")

  local locales, locale_error = i18n.locales(root)
  if not locales then return false, locale_error end

  local ok, message = command.run(root, "rm -rf " .. command.quote(destination) .. " && mkdir -p " .. command.quote(destination), true)
  if not ok then return false, message end

  local listing, list_error = command.capture(root, "find pages/templates -type f -name '*.md' -printf '%P\\n' | sort")
  if not listing then return false, list_error end

  local templates = {}
  for relative in (listing .. "\n"):gmatch("(.-)\n") do
    if relative ~= "" then templates[#templates + 1] = relative end
  end

  for _, locale in ipairs(locales) do
    for _, relative in ipairs(templates) do
      local source_path = root .. "/pages/templates/" .. relative
      local template, read_error = filesystem.read(source_path)
      if not template then return false, read_error end
      local rendered, render_error = i18n.text(root, template, locale.key)
      if not rendered then return false, render_error .. " while rendering " .. relative end

      local target_relative = template_destination(relative)
      local target = destination .. "/" .. locale.key .. "/" .. target_relative
      local parent = target:match("^(.*)/[^/]+$")
      local made, mkdir_error = command.run(root, "mkdir -p " .. command.quote(parent), true)
      if not made then return false, mkdir_error end
      local written, write_error = filesystem.write(target, rendered)
      if not written then return false, write_error end
    end
  end

  local index = { "# Documentation", "" }
  for _, locale in ipairs(locales) do
    index[#index + 1] = string.format("- [%s (`%s`)](%s/index.md)", locale.language, locale.key, locale.key)
  end
  index[#index + 1] = ""
  local written, write_error = filesystem.write(destination .. "/README.md", table.concat(index, "\n"))
  if not written then return false, write_error end

  return true, locales
end

return pages
