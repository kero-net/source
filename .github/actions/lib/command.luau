--!nonstrict
local runtime = require("./legacy-runtime")
local io, os, arg = runtime.io, runtime.os, runtime.arg
local command = {}

local process = require("@lune/process")
local is_windows = process.os == "windows"
local shell = if is_windows then (process.env.KERO_POSIX_SHELL or "C:/Program Files/Git/bin/sh.exe") else "sh"

local function invoke(root, program, live)
  local line = "cd " .. command.quote(root) .. " && " .. program
  if program:match("^cargo ") then
    line = "cd " .. command.quote(root) .. " && CARGO_TARGET_DIR=.heap/cache/build/cargo/target " .. program
  end
  return process.exec(shell, { "-c", line }, if live then { stdio = "inherit" } else nil)
end

function command.quote(value)
  return "'" .. tostring(value):gsub("'", "'\\''") .. "'"
end

function command.run(root, program, quiet)
  print("+ " .. program)
  local result = invoke(root, program, not quiet)
  if quiet and result.stderr ~= "" and not result.ok then warn(result.stderr) end
  if result.ok then return true end
  return false, string.format("command failed (exit %s): %s", tostring(result.code), program)
end

function command.run_secret(root, program, label)
  local result = invoke(root, program)
  if result.ok then return true end
  return false, string.format("%s failed (exit %s)", label or "secret command", tostring(result.code))
end

function command.capture(root, program)
  local result = invoke(root, program)
  if result.ok then return result.stdout:gsub("%s+$", "") end
  return nil, string.format("command failed (exit %s): %s\n%s", tostring(result.code), program, result.stderr)
end

return command
