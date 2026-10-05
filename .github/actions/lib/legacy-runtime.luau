--!nonstrict
-- Lua 5.4 file and process adapters for the source toolchain during its Luau transition.
local fs = require("@lune/fs")
local process = require("@lune/process")
local shell = if process.os == "windows" then (process.env.KERO_POSIX_SHELL or "C:/Program Files/Git/bin/sh.exe") else "sh"

local runtime = {}
runtime.arg = { [0] = "" }
for index, value in ipairs(process.args) do runtime.arg[index] = value end

local function stream(path, mode)
  local contents = if fs.isFile(path) then fs.readFile(path) else ""
  if mode:find("r") and not fs.isFile(path) then return nil, "missing " .. path end
  if mode:find("w") then contents = "" end
  local cursor = 1
  local handle = {}
  function handle:read(format)
    if format == "*a" then local result = contents:sub(cursor); cursor = #contents + 1; return result end
    local start, finish = contents:find("\r?\n", cursor)
    if not start then
      if cursor > #contents then return nil end
      local result = contents:sub(cursor); cursor = #contents + 1; return result
    end
    local result = contents:sub(cursor, start - 1); cursor = finish + 1; return result
  end
  function handle:lines() return function() return self:read("*l") end end
  function handle:write(...)
    for _, value in ipairs({ ... }) do contents ..= tostring(value) end
    return self
  end
  function handle:flush() end
  function handle:close()
    if mode:find("w") or mode:find("a") then fs.writeFile(path, contents) end
    return true
  end
  return handle
end

runtime.io = {
  open = stream,
  stdout = { write = function(_, ...) print(table.concat({ ... }, "")) end, flush = function() end },
  stderr = { write = function(_, ...) warn(table.concat({ ... }, "")) end },
}
function runtime.io.popen(line)
  local result = process.exec(shell, { "-c", line })
  local handle = {}
  local value = result.stdout
  local cursor = 1
  function handle:read() local output = value:sub(cursor); cursor = #value + 1; return output end
  function handle:lines()
    return function()
      local start, finish = value:find("\r?\n", cursor)
      if not start then if cursor > #value then return nil end; local last = value:sub(cursor); cursor = #value + 1; return last end
      local current = value:sub(cursor, start - 1); cursor = finish + 1; return current
    end
  end
  function handle:close() return result.ok, "exit", result.code end
  return handle
end

runtime.os = {
  getenv = function(name) return process.env[name] end,
  exit = function(code) process.exit(code or 0) end,
  execute = function(line)
    local result = process.exec(if process.os == "windows" then "cmd.exe" else shell,
      if process.os == "windows" then { "/d", "/s", "/c", line } else { "-c", line })
    if result.stdout ~= "" then print(result.stdout) end
    if result.stderr ~= "" then warn(result.stderr) end
    return result.ok, "exit", result.code
  end,
  remove = function(path) if fs.isFile(path) then fs.removeFile(path) end; return true end,
  rename = function(from, to)
    if from == to then return fs.isFile(from) or fs.isDir(from) end
    fs.move(from, to, true); return true
  end,
  tmpname = function() return ".heap/build/tmp-" .. tostring(math.random(100000000, 999999999)) end,
}
return runtime
