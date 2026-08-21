local command = {}

function command.quote(value)
  return "'" .. tostring(value):gsub("'", "'\\''") .. "'"
end

local function succeeded(ok, _, code)
  return ok == true or ok == 0 or code == 0
end

function command.run(root, program, quiet)
  if not quiet then
    io.stdout:write("+ ", program, "\n")
    io.stdout:flush()
  end
  local ok, reason, code = os.execute("cd " .. command.quote(root) .. " && " .. program)
  if succeeded(ok, reason, code) then return true end
  return false, string.format("command failed (%s %s): %s", reason or "exit", code or ok or 1, program)
end

function command.run_secret(root, program, label)
  local ok, reason, code = os.execute("cd " .. command.quote(root) .. " && " .. program)
  if succeeded(ok, reason, code) then return true end
  return false, string.format("%s failed (%s %s)", label or "secret command", reason or "exit", code or ok or 1)
end

function command.capture(root, program)
  local pipe, message = io.popen("cd " .. command.quote(root) .. " && " .. program .. " 2>&1", "r")
  if not pipe then return nil, message end
  local output = pipe:read("*a")
  local ok, reason, code = pipe:close()
  if succeeded(ok, reason, code) then
    return (output:gsub("%s+$", ""))
  end
  return nil, string.format("command failed (%s %s): %s\n%s", reason or "exit", code or ok or 1, program, output)
end

return command
