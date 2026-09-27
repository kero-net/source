-- Reads the human-owned KST distribution registry.
local M = {}

local function fail(path, line, message)
  error(path .. ":" .. line .. ": " .. message)
end

local function parse(path)
  local file = assert(io.open(path, "rb"), "missing " .. path)
  local root, stack = { children = {} }, {}
  local line_number = 0
  for line in (file:read("*a") .. "\n"):gmatch("([^\n]*)\n") do
    line_number = line_number + 1
    if line ~= "" and not line:match("^\t*#") then
      local tabs, name, value = line:match("^(\t*)([%a][%w]*)(.*)$")
      if not name then fail(path, line_number, "invalid KST node") end
      if value ~= "" then
        if value:sub(1, 1) ~= " " or value:sub(2):find("^%s") then
          fail(path, line_number, "invalid KST value")
        end
        value = value:sub(2)
      else
        value = nil
      end
      local depth = #tabs
      if depth > #stack then fail(path, line_number, "indentation jumps more than one level") end
      local parent = depth == 0 and root or stack[depth]
      if not parent then fail(path, line_number, "missing parent") end
      local node = { name = name, value = value, children = {} }
      parent.children[#parent.children + 1] = node
      stack[depth + 1] = node
      for index = depth + 2, #stack do stack[index] = nil end
    end
  end
  file:close()
  return root
end

local function child(node, name)
  for _, value in ipairs(node.children) do if value.name == name then return value end end
end

local function required(node, name)
  local value = child(node, name)
  assert(value and value.value, "distribution KST is missing " .. name)
  return value.value
end

function M.read(root)
  local document = parse(root .. "/distribution/distribution.kst")
  local verification = assert(child(document, "verification"), "distribution KST is missing verification")
  local targets = {}
  for _, node in ipairs(document.children) do
    if node.name == "target" then
      local name = assert(node.value, "distribution KST target requires a name")
      assert(not targets[name], "distribution KST repeats target " .. name)
      local enabled = required(node, "enabled")
      assert(enabled == "true" or enabled == "false", "distribution KST enabled must be true or false")
      targets[name] = {
        name = name, enabled = enabled == "true", lifecycle = required(node, "lifecycle"),
        artifact = required(node, "artifact"), toolchain = required(node, "toolchain"),
        native_host = required(node, "nativeHost"),
        required_environment = required(node, "requiredEnvironment"),
      }
    end
  end
  return {
    verification = {
      checksum = required(verification, "checksum"),
      gpg_key_environment = required(verification, "gpgKeyEnvironment"),
      gpg_private_key_environment = required(verification, "gpgPrivateKeyEnvironment"),
    },
    targets = targets,
  }
end

function M.target(root, name)
  return M.read(root).targets[name]
end

function M.adapter(root, name)
  local path = root .. "/distribution.local.kst"
  local file = io.open(path, "rb")
  if not file then return nil end
  file:close()
  local document = parse(path)
  for _, node in ipairs(document.children) do
    if node.name == "adapter" and node.value == name then
      local result = {}
      for _, setting in ipairs(node.children) do result[setting.name] = setting.value end
      return result
    end
  end
  return nil
end

return M
