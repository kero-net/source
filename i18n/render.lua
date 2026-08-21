local render = {}

local registry_cache = {}
local catalog_cache = {}

local function read(path)
  local file, message = io.open(path, "rb")
  if not file then return nil, message end
  local value = file:read("*a")
  file:close()
  return value
end

local function decode_basic_string(value)
  value = value:gsub("\\n", "\n")
  value = value:gsub("\\r", "\r")
  value = value:gsub("\\t", "\t")
  value = value:gsub('\\"', '"')
  value = value:gsub("\\\\", "\\")
  return value
end

function render.locales(root)
  local path = root .. "/i18n/locales.toml"
  if registry_cache[path] then return registry_cache[path] end

  local value, message = read(path)
  if not value then return nil, message end

  local locales = {}
  local seen = {}
  for body in value:gmatch("{(.-)}") do
    local key = body:match('key%s*=%s*"([^"]+)"')
    local language = body:match('language%s*=%s*"([^"]+)"')
    if not key or not language then
      return nil, "each locale entry must define only a key and language"
    end
    if seen[key] then return nil, "duplicate locale key: " .. key end
    seen[key] = true
    locales[#locales + 1] = { key = key, language = language }
  end

  if #locales == 0 then return nil, "i18n/locales.toml contains no locales" end
  registry_cache[path] = locales
  return locales
end

local function registered(locales, key)
  for _, locale in ipairs(locales) do
    if locale.key == key then return true end
  end
  return false
end

function render.catalog(root, name)
  local path = root .. "/i18n/" .. name .. ".toml"
  if catalog_cache[path] then return catalog_cache[path] end

  local locales, locale_error = render.locales(root)
  if not locales then return nil, locale_error end

  local value, message = read(path)
  if not value then return nil, "missing localization catalog: i18n/" .. name .. ".toml" end

  local sections = {}
  local current = nil
  local line_number = 0

  for line in (value .. "\n"):gmatch("(.-)\r?\n") do
    line_number = line_number + 1
    local trimmed = line:match("^%s*(.-)%s*$")
    if trimmed ~= "" and not trimmed:match("^#") then
      local section = trimmed:match("^%[([%w_.%-]+)%.values%]$")
      if section then
        if sections[section] then
          return nil, string.format("duplicate localization section %s in %s", section, path)
        end
        sections[section] = {}
        current = sections[section]
      elseif trimmed:sub(1, 1) == "[" then
        return nil, string.format("unsupported localization section at %s:%d", path, line_number)
      else
        if not current then
          return nil, string.format("localization value before section at %s:%d", path, line_number)
        end
        local locale, raw = trimmed:match('^([%w%-]+)%s*=%s*"(.*)"$')
        if not locale then
          return nil, string.format("unsupported localization value at %s:%d", path, line_number)
        end
        if not registered(locales, locale) then
          return nil, string.format("unknown locale %s at %s:%d", locale, path, line_number)
        end
        current[locale] = decode_basic_string(raw)
      end
    end
  end

  local default_locale = locales[1].key
  for section, values in pairs(sections) do
    if values[default_locale] == nil then
      return nil, string.format("%s.%s is missing required default locale %s", name, section, default_locale)
    end
  end

  catalog_cache[path] = sections
  return sections
end

function render.resolve(root, token, locale)
  local catalog_name, section = token:match("^([%w%-]+)%.(.+)$")
  if not catalog_name then return nil, "invalid localization key: " .. token end

  local locales, locale_error = render.locales(root)
  if not locales then return nil, locale_error end
  if not registered(locales, locale) then return nil, "unknown requested locale: " .. tostring(locale) end

  local catalog, catalog_error = render.catalog(root, catalog_name)
  if not catalog then return nil, catalog_error end
  local values = catalog[section]
  if not values then return nil, "unknown localization key: " .. token end

  if values[locale] ~= nil then return values[locale] end
  for _, candidate in ipairs(locales) do
    if values[candidate.key] ~= nil then return values[candidate.key] end
  end
  return nil, "localization key has no values: " .. token
end

function render.text(root, template, locale)
  local failure = nil
  local output = template:gsub("{{%s*l10n:([%w_.%-]+)%s*}}", function(token)
    local value, message = render.resolve(root, token, locale)
    if not value then
      failure = failure or message
      return "{{ l10n:" .. token .. " }}"
    end
    return value
  end)
  if failure then return nil, failure end
  return output
end

return render
