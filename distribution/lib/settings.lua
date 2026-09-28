-- Reads the single human-owned distribution registry.
local M = {}

local function decode(text)
  local at = 1
  local function space() at = text:find("[^%s]", at) or #text + 1 end
  local value
  local function string_value()
    at = at + 1; local out = {}
    while at <= #text do
      local character = text:sub(at, at); at = at + 1
      if character == '"' then return table.concat(out) end
      if character == "\\" then
        local escaped = text:sub(at, at); at = at + 1
        local map = { ['"'] = '"', ['\\'] = '\\', ['/'] = '/', b = '\b', f = '\f', n = '\n', r = '\r', t = '\t' }
        assert(map[escaped], "distribution JSON contains an unsupported escape")
        out[#out + 1] = map[escaped]
      else out[#out + 1] = character end
    end
    error("distribution JSON has an unterminated string")
  end
  local function array()
    at = at + 1; local out = {}; space()
    if text:sub(at, at) == "]" then at = at + 1; return out end
    while true do
      out[#out + 1] = value(); space(); local delimiter = text:sub(at, at); at = at + 1
      if delimiter == "]" then return out end
      assert(delimiter == ",", "distribution JSON array is malformed"); space()
    end
  end
  local function object()
    at = at + 1; local out = {}; space()
    if text:sub(at, at) == "}" then at = at + 1; return out end
    while true do
      assert(text:sub(at, at) == '"', "distribution JSON object key is malformed")
      local key = string_value(); space(); assert(text:sub(at, at) == ":", "distribution JSON object is malformed"); at = at + 1
      out[key] = value(); space(); local delimiter = text:sub(at, at); at = at + 1
      if delimiter == "}" then return out end
      assert(delimiter == ",", "distribution JSON object is malformed"); space()
    end
  end
  function value()
    space(); local character = text:sub(at, at)
    if character == '"' then return string_value() end
    if character == "{" then return object() end
    if character == "[" then return array() end
    if text:sub(at, at + 3) == "true" then at = at + 4; return true end
    if text:sub(at, at + 4) == "false" then at = at + 5; return false end
    if text:sub(at, at + 3) == "null" then at = at + 4; return nil end
    error("distribution JSON contains an unsupported value at byte " .. at)
  end
  local result = value(); space(); assert(at > #text, "distribution JSON has trailing content"); return result
end

function M.read(root)
  local file = assert(io.open(root .. "/distribution/builds.json", "rb"), "missing distribution/builds.json")
  local document = decode(file:read("*a")); file:close()
  assert(type(document.targets) == "table" and type(document.verification) == "table", "distribution/builds.json requires targets and verification")
  local targets = {}
  for _, item in ipairs(document.targets) do
    assert(type(item.name) == "string" and not targets[item.name], "distribution target name must be unique")
    assert(type(item.enabled) == "boolean" and type(item.requiredEnvironment) == "table", "distribution target is malformed: " .. item.name)
    item.required_environment = table.concat(item.requiredEnvironment, ",")
    item.native_host = item.nativeHost
    targets[item.name] = item
  end
  return { verification = { checksum = document.verification.checksum, gpg_key_environment = document.verification.gpgKeyEnvironment, gpg_private_key_environment = document.verification.gpgPrivateKeyEnvironment }, targets = targets }
end

function M.target(root, name) return M.read(root).targets[name] end

function M.adapter(root, name)
  local path = root .. "/distribution.local.kst"; local file = io.open(path, "rb")
  if not file then return nil end; file:close()
  local content = assert(io.open(path, "rb")):read("*a")
  local current, result = false, {}
  for line in (content .. "\n"):gmatch("([^\n]*)\n") do
    local adapter = line:match("^adapter%s+(.+)$")
    if adapter then current = adapter == name
    elseif current then
      local key, item = line:match("^\t([%w]+)%s+(.+)$")
      if key then result[key] = item elseif line:match("^%S") then break end
    end
  end
  return next(result) and result or nil
end

return M
