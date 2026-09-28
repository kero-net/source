-- Minimal KST reader for human-owned operational setting schemas.
local M = {}

function M.parse_file(path)
  local file = assert(io.open(path, "rb"), "missing " .. path)
  local root, stack = { children = {} }, {}
  local line_number = 0
  for line in (file:read("*a") .. "\n"):gmatch("([^\n]*)\n") do
    line_number = line_number + 1
    if line ~= "" and not line:match("^\t*#") then
      local tabs, name, tail = line:match("^(\t*)([%a][%w]*)(.*)$")
      assert(name, path .. ":" .. line_number .. ": invalid KST node")
      local value
      if tail ~= "" then
        assert(tail:sub(1, 1) == " " and not tail:sub(2):find("^%s"),
          path .. ":" .. line_number .. ": invalid KST value")
        value = tail:sub(2)
      end
      local depth = #tabs
      assert(depth <= #stack, path .. ":" .. line_number .. ": invalid KST indentation")
      local parent = depth == 0 and root or stack[depth]
      assert(parent, path .. ":" .. line_number .. ": missing KST parent")
      local node = { name = name, value = value, children = {} }
      parent.children[#parent.children + 1] = node
      stack[depth + 1] = node
      for index = depth + 2, #stack do stack[index] = nil end
    end
  end
  file:close()
  return root
end

function M.children(node, name)
  local values = {}
  for _, child in ipairs(node.children) do if child.name == name then values[#values + 1] = child end end
  return values
end

function M.child(node, name)
  return M.children(node, name)[1]
end

return M
